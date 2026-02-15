use near_sdk::json_types::U128;
use near_sdk::{AccountId, NearToken};
use thiserror::Error;
use std::str::FromStr;

#[derive(Error, Debug)]
pub enum NearWalletError {
    #[error("Failed to create NEAR wallet: {0}")]
    CreationError(String),
    #[error("Invalid account ID: {0}")]
    InvalidAccountId(String),
    #[error("Provider error: {0}")]
    ProviderError(String),
}

/// NEAR User wallet with account management
#[derive(Debug, Clone)]
pub struct NearUserWallet {
    /// NEAR account ID
    pub account_id: AccountId,
    /// Private key for signing transactions
    private_key: [u8; 32],
}

impl NearUserWallet {
    /// Create a new NEAR wallet with random keypair
    pub fn create_new() -> Result<Self, NearWalletError> {
        use ed25519_dalek::{Keypair, PublicKey, Signer, SecretKey};
        use rand::rngs::OsRng;
        
        let mut csprng = OsRng {};
        let keypair: Keypair = Keypair::generate(&mut csprng);
        
        let private_key: [u8; 32] = keypair.secret.as_bytes().to_owned();
        let public_key = PublicKey::from(&keypair.secret);
        
        // Generate account ID from public key (simplified approach)
        // In production, you'd want a more sophisticated account naming system
        let account_suffix = hex::encode(public_key.as_bytes())[..8].to_lowercase();
        let account_id_str = format!("user_{}.testnet", account_suffix);
        
        let account_id = AccountId::from_str(&account_id_str)
            .map_err(|e| NearWalletError::InvalidAccountId(e.to_string()))?;
        
        Ok(Self { 
            account_id,
            private_key,
        })
    }
    
    /// Create wallet from existing account ID and private key
    pub fn from_account_and_key(account_id: &str, private_key_bytes: &[u8; 32]) -> Result<Self, NearWalletError> {
        let account_id = AccountId::from_str(account_id)
            .map_err(|e| NearWalletError::InvalidAccountId(e.to_string()))?;
            
        Ok(Self {
            account_id,
            private_key: *private_key_bytes,
        })
    }
    
    /// Get the account ID as string
    pub fn account_id_string(&self) -> String {
        self.account_id.to_string()
    }
    
    /// Get the private key bytes (for encrypted storage)
    pub fn private_key_bytes(&self) -> [u8; 32] {
        self.private_key
    }
    
    /// Format NEAR balance as human-readable string
    pub fn format_balance(balance: NearToken) -> String {
        let yoctos = balance.as_yoctonear();
        let balance_str = yoctos.to_string();
        let len = balance_str.len();
        
        if len <= 24 {
            // Less than 1 NEAR
            let zeros = "0".repeat(24 - len);
            let full = format!("0.{}{}", zeros, balance_str);
            format!("{:.6}", full.parse::<f64>().unwrap_or(0.0))
        } else {
            // More than 1 NEAR
            let integer_part = &balance_str[..len - 24];
            let decimal_part = &balance_str[len - 24..len - 18]; // Show 6 decimals
            format!("{}.{}", integer_part, decimal_part)
        }
    }
    
    /// Parse NEAR amount from string
    pub fn parse_near_amount(amount_str: &str) -> Result<NearToken, NearWalletError> {
        let amount: f64 = amount_str.parse()
            .map_err(|_| NearWalletError::CreationError("Invalid amount".to_string()))?;
        
        let yoctos = (amount * 1e24) as u128;
        Ok(NearToken::from_yoctonear(yoctos))
    }
    
    /// Generate a human-readable account name suggestion
    pub fn generate_account_name(phone: &str) -> String {
        // Clean phone number and create account name
        let clean_phone = phone.replace('+', "").replace('-', "").replace(' ', "");
        let phone_suffix = if clean_phone.len() > 6 {
            &clean_phone[clean_phone.len() - 6..]
        } else {
            &clean_phone
        };
        
        format!("sms_{}.testnet", phone_suffix)
    }
    
    /// Validate NEAR account ID format
    pub fn validate_account_id(account_id: &str) -> bool {
        AccountId::from_str(account_id).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_near_wallet() {
        let wallet = NearUserWallet::create_new().unwrap();
        // Account ID should end with .testnet
        assert!(wallet.account_id_string().ends_with(".testnet"));
    }

    #[test]
    fn test_format_balance() {
        // 1 NEAR
        let one_near = NearToken::from_near(1);
        let formatted = NearUserWallet::format_balance(one_near);
        assert!(formatted.starts_with("1."));
        
        // 0.5 NEAR
        let half_near = NearToken::from_yoctonear(500000000000000000000000);
        let formatted = NearUserWallet::format_balance(half_near);
        assert_eq!(formatted, "0.500000");
    }

    #[test]
    fn test_parse_near_amount() {
        let amount = NearUserWallet::parse_near_amount("1.5").unwrap();
        assert_eq!(amount.as_yoctonear(), 1500000000000000000000000u128);
    }

    #[test]
    fn test_generate_account_name() {
        let name = NearUserWallet::generate_account_name("+1234567890");
        assert!(name.starts_with("sms_"));
        assert!(name.ends_with(".testnet"));
    }

    #[test]
    fn test_validate_account_id() {
        assert!(NearUserWallet::validate_account_id("user.testnet"));
        assert!(NearUserWallet::validate_account_id("sms_123456.testnet"));
        assert!(!NearUserWallet::validate_account_id("invalid"));
        assert!(!NearUserWallet::validate_account_id("user@invalid.testnet"));
    }
}
