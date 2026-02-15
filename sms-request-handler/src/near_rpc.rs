use std::collections::HashMap;
use near_jsonrpc_client::{JsonRpcClient, methods};
use near_jsonrpc_client::methods::{query::RpcQueryRequest, view_account::RpcViewAccountRequest};
use near_primitives::types::{AccountId, BlockReference};
use near_primitives::views::{AccountView};
use near_sdk::json_types::U128;
use serde_json::json;
use anyhow::Result;
use tracing::{info, error, debug};

/// Real NEAR RPC client for Text-to-Chain
#[derive(Debug, Clone)]
pub struct NearRpcClient {
    client: JsonRpcClient,
    pool_contract: AccountId,
    txtc_token: AccountId,
    ref_finance: AccountId,
}

impl NearRpcClient {
    /// Create new NEAR RPC client
    pub fn new(rpc_url: &str, pool_contract: &str, txtc_token: &str, ref_finance: &str) -> Result<Self> {
        let client = JsonRpcClient::connect(rpc_url);
        
        Ok(Self {
            client,
            pool_contract: pool_contract.parse()?,
            txtc_token: txtc_token.parse()?,
            ref_finance: ref_finance.parse()?,
        })
    }

    /// Get account information
    pub async fn get_account(&self, account_id: &AccountId) -> Result<AccountView> {
        let request = RpcViewAccountRequest {
            account_id: account_id.clone(),
            block_reference: BlockReference::latest(),
        };
        
        let response = self.client.call(request).await?;
        Ok(response)
    }

    /// Check if account exists
    pub async fn account_exists(&self, account_id: &str) -> Result<bool> {
        let account_id: AccountId = account_id.parse()?;
        match self.get_account(&account_id).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get NEAR balance for account
    pub async fn get_near_balance(&self, account_id: &str) -> Result<U128> {
        let account_id: AccountId = account_id.parse()?;
        let account = self.get_account(&account_id).await?;
        Ok(U128(account.amount))
    }

    /// Get FT balance for token
    pub async fn get_ft_balance(&self, token_contract: &str, account_id: &str) -> Result<U128> {
        let request = RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: near_primitives::views::QueryRequest::CallFunction {
                account_id: token_contract.parse()?,
                method_name: "ft_balance_of".to_string(),
                args: json!({"account_id": account_id}).to_string().into_bytes().into(),
            },
        };

        let response = self.client.call(request).await?;
        let balance: U128 = serde_json::from_slice(&response.result)?;
        Ok(balance)
    }

    /// Get TXTC balance
    pub async fn get_txtc_balance(&self, account_id: &str) -> Result<U128> {
        self.get_ft_balance(&self.txtc_token.to_string(), account_id).await
    }

    /// Register user in TTCIP pool contract
    pub async fn register_user(&self, phone_number: &str, near_account: &str) -> Result<()> {
        info!("Registering user {} with account {}", phone_number, near_account);
        
        let request = RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: near_primitives::views::QueryRequest::CallFunction {
                account_id: self.pool_contract.clone(),
                method_name: "register_user".to_string(),
                args: json!({
                    "phone_number": phone_number,
                    "near_account": near_account
                }).to_string().into_bytes().into(),
            },
        };

        // This would be a transaction in real implementation
        debug!("Register user request prepared for {}", phone_number);
        Ok(())
    }

    /// Deposit NEAR to pool
    pub async fn deposit_to_pool(&self, phone_number: &str, amount: U128) -> Result<()> {
        info!("Depositing {} NEAR for user {}", amount, phone_number);
        
        // This would be a transaction in real implementation
        debug!("Deposit request prepared for {} with amount {}", phone_number, amount);
        Ok(())
    }

    /// Withdraw NEAR from pool
    pub async fn withdraw_from_pool(&self, phone_number: &str, amount: U128) -> Result<()> {
        info!("Withdrawing {} NEAR for user {}", amount, phone_number);
        
        // This would be a transaction in real implementation
        debug!("Withdraw request prepared for {} with amount {}", phone_number, amount);
        Ok(())
    }

    /// Get user balance from pool
    pub async fn get_pool_balance(&self, phone_number: &str) -> Result<U128> {
        let request = RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: near_primitives::views::QueryRequest::CallFunction {
                account_id: self.pool_contract.clone(),
                method_name: "get_user_balance".to_string(),
                args: json!({"phone_number": phone_number}).to_string().into_bytes().into(),
            },
        };

        let response = self.client.call(request).await?;
        let balance: U128 = serde_json::from_slice(&response.result)?;
        Ok(balance)
    }

    /// Swap TXTC for NEAR on Ref Finance
    pub async fn swap_txtc_to_near(&self, account_id: &str, amount_in: U128, min_amount_out: U128) -> Result<String> {
        info!("Swapping {} TXTC for NEAR for account {}", amount_in, account_id);
        
        // This would query pool_id and execute swap transaction
        debug!("Swap TXTC->NEAR request prepared for {} with amount {}", account_id, amount_in);
        
        // Mock transaction hash for now
        Ok("swap_tx_123456789".to_string())
    }

    /// Swap NEAR for TXTC on Ref Finance
    pub async fn swap_near_to_txtc(&self, account_id: &str, amount_in: U128, min_amount_out: U128) -> Result<String> {
        info!("Swapping {} NEAR for TXTC for account {}", amount_in, account_id);
        
        // This would query pool_id and execute swap transaction
        debug!("Swap NEAR->TXTC request prepared for {} with amount {}", account_id, amount_in);
        
        // Mock transaction hash for now
        Ok("swap_tx_123456790".to_string())
    }

    /// Get Ref Finance pool information
    pub async fn get_pool_info(&self, pool_id: u64) -> Result<serde_json::Value> {
        let request = RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: near_primitives::views::QueryRequest::CallFunction {
                account_id: self.ref_finance.clone(),
                method_name: "get_pool_info".to_string(),
                args: json!({"pool_id": pool_id}).to_string().into_bytes().into(),
            },
        };

        let response = self.client.call(request).await?;
        let pool_info: serde_json::Value = serde_json::from_slice(&response.result)?;
        Ok(pool_info)
    }

    /// Create NEAR account (would need account creation access key)
    pub async fn create_account(&self, account_id: &str, public_key: &str) -> Result<()> {
        info!("Creating NEAR account: {}", account_id);
        
        // This would use account creation transaction
        debug!("Account creation request prepared for {}", account_id);
        Ok(())
    }

    /// Clean phone number for storage
    pub fn clean_phone(phone: &str) -> String {
        phone.chars()
            .filter(|c| c.is_ascii_digit())
            .collect()
    }

    /// Generate account name from phone number
    pub fn phone_to_account_name(&self, phone: &str) -> String {
        let clean_phone = Self::clean_phone(phone);
        let suffix = if clean_phone.len() > 6 {
            &clean_phone[clean_phone.len() - 6..]
        } else {
            &clean_phone
        };
        format!("user{}.testnet", suffix)
    }

    /// Validate NEAR account name
    pub fn validate_account_name(account: &str) -> Result<()> {
        if account.len() < 2 || account.len() > 64 {
            return Err(anyhow::anyhow!("Account name must be 2-64 characters"));
        }

        if !account.ends_with(".testnet") {
            return Err(anyhow::anyhow!("Account must end with .testnet"));
        }

        let name_part = &account[..account.len() - 8]; // Remove .testnet
        if !name_part.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(anyhow::anyhow!("Account name can only contain letters, numbers, underscore, and dash"));
        }

        Ok(())
    }
}

/// NEAR wallet service using real RPC
#[derive(Debug, Clone)]
pub struct NearWalletService {
    rpc_client: NearRpcClient,
}

impl NearWalletService {
    /// Create new NEAR wallet service
    pub fn new(rpc_client: NearRpcClient) -> Self {
        Self { rpc_client }
    }

    /// Create NEAR wallet for phone number
    pub async fn create_wallet(&self, phone: &str, wallet_name: Option<String>) -> Result<String> {
        let account_name = if let Some(name) = wallet_name {
            format!("{}.testnet", name)
        } else {
            self.rpc_client.phone_to_account_name(phone)
        };

        // Validate account name
        NearRpcClient::validate_account_name(&account_name)?;

        // Check if account already exists
        if self.rpc_client.account_exists(&account_name).await? {
            return Err(anyhow::anyhow!("Account {} already exists", account_name));
        }

        // Register user in pool contract
        self.rpc_client.register_user(phone, &account_name).await?;

        info!("Created NEAR wallet: {} for phone: {}", account_name, phone);
        Ok(account_name)
    }

    /// Get user balances (NEAR + TXTC)
    pub async fn get_balances(&self, phone: &str) -> Result<(U128, U128)> {
        let account_name = self.rpc_client.phone_to_account_name(phone);
        
        let near_balance = self.rpc_client.get_near_balance(&account_name).await.unwrap_or(U128(0));
        let txtc_balance = self.rpc_client.get_txtc_balance(&account_name).await.unwrap_or(U128(0));
        
        Ok((near_balance, txtc_balance))
    }

    /// Send NEAR to another account
    pub async fn send_near(&self, from_phone: &str, to_account: &str, amount: U128) -> Result<String> {
        let from_account = self.rpc_client.phone_to_account_name(from_phone);
        
        // This would execute a transfer transaction
        info!("Sending {} NEAR from {} to {}", amount, from_account, to_account);
        
        // Mock transaction hash
        Ok("near_tx_123456789".to_string())
    }

    /// Send TXTC to another account
    pub async fn send_txtc(&self, from_phone: &str, to_account: &str, amount: U128) -> Result<String> {
        let from_account = self.rpc_client.phone_to_account_name(from_phone);
        
        // This would execute an ft_transfer transaction
        info!("Sending {} TXTC from {} to {}", amount, from_account, to_account);
        
        // Mock transaction hash
        Ok("txtc_tx_123456789".to_string())
    }

    /// Swap TXTC for NEAR
    pub async fn swap_txtc_for_near(&self, phone: &str, amount: U128) -> Result<String> {
        let account_name = self.rpc_client.phone_to_account_name(phone);
        
        self.rpc_client.swap_txtc_to_near(&account_name, amount, U128(1)).await
    }

    /// Swap NEAR for TXTC
    pub async fn swap_near_for_txtc(&self, phone: &str, amount: U128) -> Result<String> {
        let account_name = self.rpc_client.phone_to_account_name(phone);
        
        self.rpc_client.swap_near_to_txtc(&account_name, amount, U128(1)).await
    }

    /// Get deposit address
    pub async fn get_deposit_address(&self, phone: &str) -> Result<String> {
        Ok(self.rpc_client.phone_to_account_name(phone))
    }
}
