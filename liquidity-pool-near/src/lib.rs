// TTCIP Pool - SMS-based DeFi Pool Contract
// EVM parity: VoucherManager + Uniswap-style TXTC-NEAR swap pool
use near_sdk::json_types::U128;
use near_sdk::{AccountId, Gas, NearToken, PanicOnDefault, Promise, env, near, require};
use std::collections::HashMap;
use tiny_keccak::{Hasher, Keccak};

const GAS_FT_TRANSFER: Gas = Gas::from_tgas(50);
const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);
const FEE_DENOMINATOR: u128 = 10_000;

/// Voucher record (EVM VoucherManager parity)
#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct VoucherInfo {
    pub amount: u128,
    pub redeemed: bool,
}

/// External TXTC (NEP-141) for sending tokens from pool
#[near_sdk::ext_contract(ext_txtc)]
pub trait ExtTxtc {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

// Define the contract structure for SMS-based DeFi pool
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    // Pool management
    owner: AccountId,
    total_liquidity: NearToken,

    // User balances (phone_number -> balance)
    user_balances: HashMap<String, NearToken>,

    // Phone number to NEAR account mapping
    phone_to_account: HashMap<String, AccountId>,

    // Pool configuration
    is_paused: bool,
    min_deposit: NearToken,

    // TXTC-NEAR swap pool (Uniswap-style constant product)
    txtc_token_id: AccountId,
    txtc_reserve: u128,
    near_reserve: u128, // yoctoNEAR for AMM
    swap_fee_bps: u16, // e.g. 30 = 0.3%

    // Voucher redemption (EVM VoucherManager parity)
    vouchers: HashMap<[u8; 32], VoucherInfo>,
}

// User balance structure for JSON responses
#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct UserBalance {
    pub phone_number: String,
    pub balance: NearToken,
    pub near_account: Option<AccountId>,
}

// Transaction record for tracking
#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: NearToken,
    pub transaction_type: String, // "DEPOSIT", "WITHDRAW", "TRANSFER", "SWAP"
    pub timestamp: u64,
}

// Event for liquidity added
#[near(event_json(standard = "ttcip-pool"))]
pub enum PoolEvent {
    #[event_version("1.0.0")]
    LiquidityAdded {
        provider: AccountId,
        amount_near: U128,
        amount_txtc: U128,
    },
}

// Implement the contract functions
#[near]
impl Contract {
    /// Initialize the TTCIP pool contract
    #[init]
    pub fn init(owner: AccountId, min_deposit: U128, txtc_token_id: AccountId) -> Self {
        Self {
            owner,
            total_liquidity: NearToken::from_yoctonear(0),
            user_balances: HashMap::new(),
            phone_to_account: HashMap::new(),
            is_paused: false,
            min_deposit: NearToken::from_yoctonear(min_deposit.into()),
            txtc_token_id,
            txtc_reserve: 0,
            near_reserve: 0,
            swap_fee_bps: 30,
            vouchers: HashMap::new(),
        }
    }

    // --- Voucher redemption (EVM VoucherManager parity) ---

    /// Add voucher - owner only. Uses keccak256(code) for EVM parity.
    pub fn add_voucher(&mut self, code: String, amount: U128) {
        require!(env::predecessor_account_id() == self.owner, "Only owner");
        require!(amount.0 > 0, "Zero amount");
        let hash = Self::keccak256(code.as_bytes());
        require!(!self.vouchers.contains_key(&hash), "Voucher exists");
        self.vouchers.insert(hash, VoucherInfo { amount: amount.0, redeemed: false });
    }

    /// Redeem voucher - transfers TXTC from pool to recipient (EVM redeemVoucher parity)
    pub fn redeem_voucher(&mut self, code: String, recipient: AccountId) -> Promise {
        require!(!self.is_paused, "Pool paused");
        require!(recipient != env::current_account_id(), "Invalid recipient");
        let hash = Self::keccak256(code.as_bytes());
        let v = self.vouchers.get_mut(&hash).expect("Voucher not found");
        require!(!v.redeemed, "Already redeemed");
        v.redeemed = true;
        let amount = v.amount;
        require!(self.txtc_reserve >= amount, "Insufficient pool TXTC");
        self.txtc_reserve -= amount;
        ext_txtc::ext(self.txtc_token_id.clone())
            .with_attached_deposit(ONE_YOCTO)
            .with_static_gas(GAS_FT_TRANSFER)
            .ft_transfer(recipient, U128(amount), Some("voucher_redeem".to_string()))
    }

    fn keccak256(data: &[u8]) -> [u8; 32] {
        let mut out = [0u8; 32];
        let mut h = Keccak::v256();
        h.update(data);
        h.finalize(&mut out);
        out
    }

    /// Get voucher info by code (hashes on-chain)
    pub fn get_voucher(&self, code: String) -> Option<VoucherInfo> {
        let hash = Self::keccak256(code.as_bytes());
        self.vouchers.get(&hash).cloned()
    }

    // --- TXTC-NEAR swap pool (Uniswap-style) ---

    /// Swap NEAR for TXTC. User attaches NEAR, receives TXTC from pool.
    #[payable]
    pub fn swap_near_for_txtc(&mut self, min_txtc_out: U128) -> Promise {
        require!(!self.is_paused, "Pool paused");
        let near_in = env::attached_deposit();
        require!(near_in > NearToken::from_yoctonear(0), "Zero NEAR");
        let near_in_u = near_in.as_yoctonear();
        let (txtc_out, _) = Self::compute_swap_near_to_txtc(
            self.near_reserve,
            self.txtc_reserve,
            near_in_u,
            self.swap_fee_bps,
        );
        require!(txtc_out >= min_txtc_out.0, "Slippage");
        self.near_reserve += near_in_u;
        self.txtc_reserve -= txtc_out;
        ext_txtc::ext(self.txtc_token_id.clone())
            .with_attached_deposit(ONE_YOCTO)
            .with_static_gas(GAS_FT_TRANSFER)
            .ft_transfer(env::predecessor_account_id(), U128(txtc_out), Some("swap".to_string()))
    }

    /// Called when user sends TXTC via ft_transfer_call (msg: "swap" or "add_liquidity")
    pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> U128 {
        require!(env::predecessor_account_id() == self.txtc_token_id, "Invalid token");
        require!(!self.is_paused, "Pool paused");
        let amount_u = amount.0;
        require!(amount_u > 0, "Zero amount");

        if msg == "add_liquidity" {
            self.txtc_reserve += amount_u;
            PoolEvent::LiquidityAdded {
                provider: sender_id,
                amount_near: U128(0),
                amount_txtc: amount,
            }
            .emit();
            return U128(0);
        }

        if msg == "swap" || msg.is_empty() {
            let (near_out, _) = Self::compute_swap_txtc_to_near(
                self.txtc_reserve,
                self.near_reserve,
                amount_u,
                self.swap_fee_bps,
            );
            require!(self.near_reserve >= near_out, "Insufficient NEAR reserve");
            self.txtc_reserve += amount_u;
            self.near_reserve -= near_out;
            let _ = Promise::new(sender_id).transfer(NearToken::from_yoctonear(near_out));
            return U128(0);
        }

        U128(amount_u)
    }

    fn compute_swap_near_to_txtc(near_reserve: u128, txtc_reserve: u128, near_in: u128, fee_bps: u16) -> (u128, u128) {
        let fee = (near_in as u128 * fee_bps as u128) / FEE_DENOMINATOR;
        let near_in_after_fee = near_in - fee;
        let denominator = near_reserve + near_in_after_fee;
        
        if denominator == 0 {
            return (0, fee);
        }
        
        // Avoid overflow: (txtc_reserve * near_in_after_fee) / denominator
        // Rewrite as: txtc_reserve * (near_in_after_fee / denominator) + remainder handling
        let txtc_out = txtc_reserve
            .checked_mul(near_in_after_fee)
            .map(|n| n / denominator)
            .unwrap_or_else(|| {
                // Overflow case: do division first
                let quotient = near_in_after_fee / denominator;
                txtc_reserve * quotient
            });
        
        (txtc_out, fee)
    }

    fn compute_swap_txtc_to_near(txtc_reserve: u128, near_reserve: u128, txtc_in: u128, fee_bps: u16) -> (u128, u128) {
        let fee = (txtc_in as u128 * fee_bps as u128) / FEE_DENOMINATOR;
        let txtc_in_after_fee = txtc_in - fee;
        let denominator = txtc_reserve + txtc_in_after_fee;
        
        if denominator == 0 {
            return (0, fee);
        }
        
        // Avoid overflow similar to above
        let near_out = near_reserve
            .checked_mul(txtc_in_after_fee)
            .map(|n| n / denominator)
            .unwrap_or_else(|| {
                let quotient = txtc_in_after_fee / denominator;
                near_reserve * quotient
            });
        
        (near_out, fee)
    }

    /// Add NEAR liquidity to the swap pool
    #[payable]
    pub fn add_liquidity_near(&mut self) {
        require!(env::predecessor_account_id() == self.owner, "Only owner");
        let amount = env::attached_deposit();
        require!(amount > NearToken::from_yoctonear(0), "Zero NEAR");
        require!(amount > NearToken::from_yoctonear(0), "Zero NEAR");
        self.near_reserve += amount.as_yoctonear();
        PoolEvent::LiquidityAdded {
            provider: env::predecessor_account_id(),
            amount_near: U128(amount.as_yoctonear()),
            amount_txtc: U128(0),
        }
        .emit();
    }

    /// Add TXTC liquidity via ft_transfer_call with msg "add_liquidity"
    /// (txtc_reserve is updated in ft_on_transfer)

    /// Get pool reserves for swap quotes
    pub fn get_pool_reserves(&self) -> (U128, U128) {
        (U128(self.txtc_reserve), U128(self.near_reserve))
    }

    /// Quote: NEAR in -> TXTC out
    pub fn quote_near_to_txtc(&self, near_in: U128) -> U128 {
        let (out, _) = Self::compute_swap_near_to_txtc(
            self.near_reserve,
            self.txtc_reserve,
            near_in.0,
            self.swap_fee_bps,
        );
        U128(out)
    }

    /// Quote: TXTC in -> NEAR out
    pub fn quote_txtc_to_near(&self, txtc_in: U128) -> U128 {
        let (out, _) = Self::compute_swap_txtc_to_near(
            self.txtc_reserve,
            self.near_reserve,
            txtc_in.0,
            self.swap_fee_bps,
        );
        U128(out)
    }

    /// Register a user with phone number and NEAR account
    pub fn register_user(&mut self, phone_number: String, near_account: AccountId) {
        require!(!self.is_paused, "Pool is currently paused");
        require!(!self.phone_to_account.contains_key(&phone_number), "User already registered");
        
        self.phone_to_account.insert(phone_number.clone(), near_account.clone());
        self.user_balances.insert(phone_number, NearToken::from_yoctonear(0));
    }

    /// Deposit NEAR tokens to the pool (called by SMS handler)
    #[payable]
    pub fn deposit(&mut self, phone_number: String) {
        require!(!self.is_paused, "Pool is currently paused");
        require!(self.phone_to_account.contains_key(&phone_number), "User not registered");
        
        let deposit_amount = env::attached_deposit();
        require!(deposit_amount >= self.min_deposit, "Deposit below minimum");
        
        let zero_balance = NearToken::from_yoctonear(0);
        let current_balance = self.user_balances.get(&phone_number).unwrap_or(&zero_balance);
        let new_balance = current_balance.saturating_add(deposit_amount);
        
        self.user_balances.insert(phone_number, new_balance);
        self.total_liquidity = self.total_liquidity.saturating_add(deposit_amount);
    }

    /// Withdraw NEAR tokens from the pool
    pub fn withdraw(&mut self, phone_number: String, amount: U128) -> Promise {
        require!(!self.is_paused, "Pool is currently paused");
        require!(self.phone_to_account.contains_key(&phone_number), "User not registered");
        
        let withdraw_amount = NearToken::from_yoctonear(amount.into());
        let zero_balance = NearToken::from_yoctonear(0);
        let current_balance = self.user_balances.get(&phone_number).unwrap_or(&zero_balance);
        
        require!(*current_balance >= withdraw_amount, "Insufficient balance");
        
        let new_balance = current_balance.saturating_sub(withdraw_amount);
        self.user_balances.insert(phone_number.clone(), new_balance);
        self.total_liquidity = self.total_liquidity.saturating_sub(withdraw_amount);
        
        let near_account = self.phone_to_account.get(&phone_number).unwrap();
        Promise::new(near_account.clone()).transfer(withdraw_amount)
    }

    /// Transfer tokens between users
    pub fn transfer(&mut self, from_phone: String, to_phone: String, amount: U128) {
        require!(!self.is_paused, "Pool is currently paused");
        require!(self.phone_to_account.contains_key(&from_phone), "Sender not registered");
        require!(self.phone_to_account.contains_key(&to_phone), "Recipient not registered");
        
        let transfer_amount = NearToken::from_yoctonear(amount.into());
        let zero_balance = NearToken::from_yoctonear(0);
        let from_balance = self.user_balances.get(&from_phone).unwrap_or(&zero_balance);
        
        require!(*from_balance >= transfer_amount, "Insufficient balance");
        
        // Update balances
        let new_from_balance = from_balance.saturating_sub(transfer_amount);
        let zero_balance = NearToken::from_yoctonear(0);
        let to_balance = self.user_balances.get(&to_phone).unwrap_or(&zero_balance);
        let new_to_balance = to_balance.saturating_add(transfer_amount);
        
        self.user_balances.insert(from_phone, new_from_balance);
        self.user_balances.insert(to_phone, new_to_balance);
    }

    /// Get user balance
    pub fn get_balance(&self, phone_number: String) -> UserBalance {
        let zero_balance = NearToken::from_yoctonear(0);
        let balance = self.user_balances.get(&phone_number).unwrap_or(&zero_balance);
        let near_account = self.phone_to_account.get(&phone_number).cloned();
        
        UserBalance {
            phone_number,
            balance: *balance,
            near_account,
        }
    }

    /// Get total pool liquidity
    pub fn get_total_liquidity(&self) -> NearToken {
        self.total_liquidity
    }

    /// Check if user is registered
    pub fn is_user_registered(&self, phone_number: String) -> bool {
        self.phone_to_account.contains_key(&phone_number)
    }

    /// Owner functions
    #[private]
    pub fn pause_pool(&mut self) {
        require!(env::predecessor_account_id() == self.owner, "Only owner can pause");
        self.is_paused = true;
    }

    #[private]
    pub fn unpause_pool(&mut self) {
        require!(env::predecessor_account_id() == self.owner, "Only owner can unpause");
        self.is_paused = false;
    }

    #[private]
    pub fn update_min_deposit(&mut self, new_min: U128) {
        require!(env::predecessor_account_id() == self.owner, "Only owner can update minimum");
        self.min_deposit = NearToken::from_yoctonear(new_min.into());
    }

    pub fn update_txtc_token(&mut self, new_token: AccountId) {
        require!(env::predecessor_account_id() == self.owner, "Only owner can update token");
        self.txtc_token_id = new_token;
    }

    /// Emergency withdraw by owner
    #[private]
    pub fn emergency_withdraw(&mut self, amount: U128) -> Promise {
        require!(env::predecessor_account_id() == self.owner, "Only owner can emergency withdraw");
        require!(self.total_liquidity >= NearToken::from_yoctonear(amount.into()), "Insufficient pool liquidity");
        
        let withdraw_amount = NearToken::from_yoctonear(amount.into());
        self.total_liquidity = self.total_liquidity.saturating_sub(withdraw_amount);
        
        Promise::new(self.owner.clone()).transfer(withdraw_amount)
    }
}

/*
 * Tests for the TTCIP Pool contract
 */
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::json_types::U128;
    use near_sdk::test_utils::{VMContextBuilder, accounts};
    use near_sdk::{AccountId, testing_env};

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn init_contract() {
        let owner: AccountId = "owner.near".parse().unwrap();
        let txtc: AccountId = "txtc.testnet".parse().unwrap();
        let min_deposit: U128 = U128::from(1000000); // 1 NEAR minimum
        let contract = Contract::init(owner.clone(), min_deposit, txtc);

        assert_eq!(contract.get_total_liquidity(), NearToken::from_yoctonear(0));
        assert!(!contract.is_user_registered("+1234567890".to_string()));
    }

    #[test]
    fn register_user() {
        let owner: AccountId = "owner.near".parse().unwrap();
        let alice: AccountId = "alice.near".parse().unwrap();
        let phone = "+1234567890".to_string();
        let min_deposit: U128 = U128::from(1000000);
        let txtc: AccountId = "txtc.testnet".parse().unwrap();
        let mut contract = Contract::init(owner, min_deposit, txtc);

        contract.register_user(phone.clone(), alice.clone());
        
        assert!(contract.is_user_registered(phone.clone()));
        let balance = contract.get_balance(phone);
        assert_eq!(balance.balance, NearToken::from_yoctonear(0));
        assert_eq!(balance.near_account.unwrap(), alice);
    }

    #[test]
    #[should_panic(expected = "User already registered")]
    fn register_duplicate_user() {
        let owner: AccountId = "owner.near".parse().unwrap();
        let alice: AccountId = "alice.near".parse().unwrap();
        let phone = "+1234567890".to_string();
        let min_deposit: U128 = U128::from(1000000);
        let txtc: AccountId = "txtc.testnet".parse().unwrap();
        let mut contract = Contract::init(owner, min_deposit, txtc);

        contract.register_user(phone.clone(), alice.clone());
        contract.register_user(phone, alice); // Should panic
    }

    #[test]
    fn deposit_success() {
        let owner: AccountId = "owner.near".parse().unwrap();
        let alice: AccountId = "alice.near".parse().unwrap();
        let phone = "+1234567890".to_string();
        let min_deposit: U128 = U128::from(1000000);
        let txtc: AccountId = "txtc.testnet".parse().unwrap();
        let mut contract = Contract::init(owner, min_deposit, txtc);

        contract.register_user(phone.clone(), alice.clone());

        // Set up context for deposit
        let mut context = get_context(alice.clone());
        context.attached_deposit(NearToken::from_near(2));
        testing_env!(context.build());

        contract.deposit(phone.clone());

        let balance = contract.get_balance(phone);
        assert_eq!(balance.balance, NearToken::from_near(2));
        assert_eq!(contract.get_total_liquidity(), NearToken::from_near(2));
    }

    #[test]
    #[should_panic(expected = "User not registered")]
    fn deposit_unregistered_user() {
        let owner: AccountId = "owner.near".parse().unwrap();
        let alice: AccountId = "alice.near".parse().unwrap();
        let phone = "+1234567890".to_string();
        let min_deposit: U128 = U128::from(1000000);
        let txtc: AccountId = "txtc.testnet".parse().unwrap();
        let mut contract = Contract::init(owner, min_deposit, txtc);

        // Don't register user

        let mut context = get_context(alice.clone());
        context.attached_deposit(NearToken::from_near(2));
        testing_env!(context.build());

        contract.deposit(phone); // Should panic
    }

    #[test]
    fn transfer_success() {
        let owner: AccountId = "owner.near".parse().unwrap();
        let alice: AccountId = "alice.near".parse().unwrap();
        let bob: AccountId = "bob.near".parse().unwrap();
        let alice_phone = "+1234567890".to_string();
        let bob_phone = "+0987654321".to_string();
        let min_deposit: U128 = U128::from(1000000);
        let txtc: AccountId = "txtc.testnet".parse().unwrap();
        let mut contract = Contract::init(owner, min_deposit, txtc);

        contract.register_user(alice_phone.clone(), alice.clone());
        contract.register_user(bob_phone.clone(), bob.clone());

        // Deposit to Alice's account
        let mut context = get_context(alice.clone());
        context.attached_deposit(NearToken::from_near(2));
        testing_env!(context.build());
        contract.deposit(alice_phone.clone());

        // Transfer from Alice to Bob
        testing_env!(get_context(alice).build());
        contract.transfer(alice_phone.clone(), bob_phone.clone(), U128::from(1000000000000000000000000)); // 1 NEAR

        let alice_balance = contract.get_balance(alice_phone);
        let bob_balance = contract.get_balance(bob_phone);
        
        assert_eq!(alice_balance.balance, NearToken::from_near(1));
        assert_eq!(bob_balance.balance, NearToken::from_near(1));
    }

    #[test]
    #[should_panic(expected = "Insufficient balance")]
    fn transfer_insufficient_balance() {
        let owner: AccountId = "owner.near".parse().unwrap();
        let alice: AccountId = "alice.near".parse().unwrap();
        let bob: AccountId = "bob.near".parse().unwrap();
        let alice_phone = "+1234567890".to_string();
        let bob_phone = "+0987654321".to_string();
        let min_deposit: U128 = U128::from(1000000);
        let txtc: AccountId = "txtc.testnet".parse().unwrap();
        let mut contract = Contract::init(owner, min_deposit, txtc);

        contract.register_user(alice_phone.clone(), alice.clone());
        contract.register_user(bob_phone.clone(), bob.clone());

        // Try to transfer without depositing
        testing_env!(get_context(alice).build());
        contract.transfer(alice_phone, bob_phone, U128::from(1000000000000000000000000)); // Should panic
    }
}
