use std::collections::HashMap;

/// Simple NEAR wallet service for SMS-based Text-to-Chain
#[derive(Debug, Clone)]
pub struct SimpleNearWalletService {
    /// User phone to NEAR account mapping
    pub phone_to_account: HashMap<String, String>,
    /// User account balances (in yoctoNEAR)
    pub user_balances: HashMap<String, u128>,
}

impl SimpleNearWalletService {
    /// Create new NEAR wallet service
    pub fn new() -> Self {
        Self {
            phone_to_account: HashMap::new(),
            user_balances: HashMap::new(),
        }
    }

    /// Clean phone number by removing common separators
    fn clean_phone(phone: &str) -> String {
        phone.chars()
            .filter(|c| c.is_ascii_digit())
            .collect()
    }

    /// Create NEAR wallet for phone number with custom name
    pub fn create_wallet(&mut self, phone: &str, wallet_name: &str) -> Result<String, String> {
        // Clean phone number
        let phone_clean = Self::clean_phone(phone);
        
        // Validate wallet name
        if wallet_name.len() < 3 || wallet_name.len() > 64 {
            return Err("Wallet name must be 3-64 characters".to_string());
        }
        
        if !wallet_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err("Wallet name can only contain letters, numbers, underscore, and dash".to_string());
        }

        // Check if phone already has a wallet
        if self.phone_to_account.contains_key(&phone_clean) {
            return Err("Phone already has a wallet".to_string());
        }

        // Create NEAR account name
        let near_account = format!("{}.testnet", wallet_name);

        // Store mapping
        self.phone_to_account.insert(phone_clean.clone(), near_account.clone());
        self.user_balances.insert(phone_clean, 0);

        Ok(format!(
            "🎉 NEAR Wallet Created!\n\n📱 Phone: {}\n🏷️  Account: {}\n🔐 PIN: 1234\n\n💰 Fund your wallet with NEAR to start using Text-to-Chain.\n\nCommands:\n• BALANCE - Check funds\n• DEPOSIT - Get deposit address\n• SEND 1 NEAR TO {}.testnet\n• SWAP 10 TXTC FOR NEAR\n• CASHOUT 10 USDC ON ARBITRUM\n• MENU - Show all commands",
            phone,
            near_account,
            wallet_name
        ))
    }

    /// Get user balance
    pub fn get_balance(&self, phone: &str) -> Result<String, String> {
        let phone_clean = Self::clean_phone(phone);
        
        let account_id = self.phone_to_account.get(&phone_clean)
            .ok_or("Wallet not found. Send JOIN to create one")?;
        
        let balance = self.user_balances.get(&phone_clean).unwrap_or(&0);
        let near_balance = *balance / 1_000_000_000_000_000_000_000_000; // Convert yoctoNEAR to NEAR
        
        Ok(format!(
            "💰 NEAR Wallet Balance\n\n🏷️  Account: {}\n💎 Balance: {} NEAR\n\n💡 Send NEAR to your account to start trading",
            account_id,
            near_balance
        ))
    }

    /// Send NEAR to another account
    pub fn send_near(&mut self, from_phone: &str, to_account: &str, amount: f64) -> Result<String, String> {
        let from_clean = Self::clean_phone(from_phone);
        
        // Validate sender has wallet
        let from_account = self.phone_to_account.get(&from_clean)
            .ok_or("Sender wallet not found")?;
        
        // Parse recipient account
        let to_account_id = format!("{}.testnet", to_account);
        
        // Convert amount to yoctoNEAR
        let amount_yocto = (amount * 1_000_000_000_000_000_000_000_000.0) as u128;
        
        // Check sender balance
        let sender_balance = self.user_balances.get(&from_clean).unwrap_or(&0);
        if *sender_balance < amount_yocto {
            return Err("Insufficient balance".to_string());
        }
        
        // Update balances (simplified - in real implementation would call NEAR contract)
        let new_balance = sender_balance - amount_yocto;
        self.user_balances.insert(from_clean.clone(), new_balance);
        
        Ok(format!(
            "✅ NEAR Transfer Sent!\n\n📤 From: {}\n📥 To: {}\n💎 Amount: {} NEAR\n🔗 TX: tx_123456\n\n💰 New Balance: {} NEAR",
            from_account,
            to_account_id,
            amount,
            new_balance / 1_000_000_000_000_000_000_000_000
        ))
    }

    /// Swap TXTC for NEAR
    pub fn swap_txtc_for_near(&mut self, phone: &str, txtc_amount: f64) -> Result<String, String> {
        let phone_clean = Self::clean_phone(phone);
        
        // Check user has wallet
        let account_id = self.phone_to_account.get(&phone_clean)
            .ok_or("Wallet not found. Send JOIN to create one")?;
        
        // Simple swap rate: 1 TXTC = 0.001 NEAR (for demo)
        let near_amount = txtc_amount * 0.001;
        let near_yocto = (near_amount * 1_000_000_000_000_000_000_000_000.0) as u128;
        
        // Add NEAR to user balance
        let current_balance = self.user_balances.get(&phone_clean).unwrap_or(&0);
        let new_balance = current_balance + near_yocto;
        self.user_balances.insert(phone_clean, new_balance);
        
        Ok(format!(
            "🔄 TXTC → NEAR Swap Complete!\n\n🏷️  Account: {}\n📤 Sent: {} TXTC\n📥 Received: {} NEAR\n💰 New Balance: {} NEAR\n\n🔗 TX: swap_789012",
            account_id,
            txtc_amount,
            near_amount,
            new_balance / 1_000_000_000_000_000_000_000_000
        ))
    }

    /// Cashout to USDC on Arbitrum
    pub fn cashout_usdc(&mut self, phone: &str, near_amount: f64) -> Result<String, String> {
        let phone_clean = Self::clean_phone(phone);
        
        // Check user has wallet
        let account_id = self.phone_to_account.get(&phone_clean)
            .ok_or("Wallet not found. Send JOIN to create one")?;
        
        // Convert NEAR to yoctoNEAR
        let near_yocto = (near_amount * 1_000_000_000_000_000_000_000_000.0) as u128;
        
        // Check balance
        let current_balance = self.user_balances.get(&phone_clean).unwrap_or(&0);
        if *current_balance < near_yocto {
            return Err("Insufficient NEAR balance".to_string());
        }
        
        // Simple cashout rate: 1 NEAR = 100 USDC (for demo)
        let usdc_amount = near_amount * 100.0;
        
        // Deduct NEAR from balance
        let new_balance = current_balance - near_yocto;
        self.user_balances.insert(phone_clean, new_balance);
        
        Ok(format!(
            "💸 USDC Cashout Initiated!\n\n🏷️  Account: {}\n📤 NEAR Sent: {} NEAR\n📥 USDC to Receive: {} USDC\n🌐 Destination: Arbitrum\n⏡ ETA: 5-10 minutes\n\n💰 New NEAR Balance: {} NEAR\n\n🔗 TX: cashout_345678",
            account_id,
            near_amount,
            usdc_amount,
            new_balance / 1_000_000_000_000_000_000_000_000
        ))
    }

    /// Get help menu
    pub fn get_help() -> String {
        "📱 Text-to-Chain NEAR Commands:\n\n🆕 JOIN <name> - Create NEAR wallet (e.g., JOIN alice)\n💰 BALANCE - Check NEAR balance\n💸 SEND <amount> NEAR TO <account> - Send NEAR\n🔄 SWAP <amount> TXTC FOR NEAR - Swap TXTC for NEAR\n💸 CASHOUT <amount> NEAR - Cashout to USDC on Arbitrum\n📥 DEPOSIT - Get deposit address\n📋 MENU - Show this help\n\n💡 Examples:\n• JOIN alice\n• SEND 1 NEAR TO bob\n• SWAP 100 TXTC FOR NEAR\n• CASHOUT 0.5 NEAR".to_string()
    }
}
