use ethers::core::k256::ecdsa::SigningKey;
use ethers::prelude::*;
use ethers::signers::Wallet;
use rand::rngs::OsRng;
use thiserror::Error;

use super::AmoyProvider;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Failed to create wallet: {0}")]
    CreationError(String),
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
}

/// User wallet with signer
#[derive(Debug, Clone)]
pub struct UserWallet {
    /// Wallet address
    pub address: Address,
    /// Private key bytes
    private_key: [u8; 32],
}

impl UserWallet {
    /// Create a new random wallet
    pub fn create_new() -> Result<Self, WalletError> {
        let wallet = Wallet::new(&mut OsRng);
        let address = wallet.address();
        let private_key: [u8; 32] = wallet.signer().to_bytes().into();
        
        Ok(Self { address, private_key })
    }

    /// Get the private key bytes (for encrypted storage)
    pub fn private_key_bytes(&self) -> [u8; 32] {
        self.private_key
    }

    /// Restore wallet from private key bytes
    pub fn from_private_key(bytes: &[u8; 32]) -> Result<Self, WalletError> {
        let signing_key = SigningKey::from_bytes(bytes.into())
            .map_err(|e| WalletError::CreationError(e.to_string()))?;
        let wallet: Wallet<SigningKey> = signing_key.into();
        let address = wallet.address();
        
        Ok(Self { 
            address,
            private_key: *bytes,
        })
    }

    /// Get the wallet address as a checksum string
    pub fn address_string(&self) -> String {
        format!("{:?}", self.address)
    }

    /// Check the native token balance (MATIC on Polygon)
    pub async fn get_balance(&self, provider: &AmoyProvider) -> Result<U256, WalletError> {
        provider
            .get_balance(self.address, None)
            .await
            .map_err(|e| WalletError::ProviderError(e.to_string()))
    }

    /// Format balance as human-readable string (in MATIC/ETH)
    pub fn format_balance(balance: U256) -> String {
        // Convert wei to ether (18 decimals)
        let wei_str = balance.to_string();
        let len = wei_str.len();
        
        if len <= 18 {
            let zeros = "0".repeat(18 - len);
            let full = format!("0.{}{}", zeros, wei_str);
            // Trim to 4 decimal places
            format!("{:.6}", full.parse::<f64>().unwrap_or(0.0))
        } else {
            let integer_part = &wei_str[..len - 18];
            let decimal_part = &wei_str[len - 18..len - 14]; // Show 4 decimals
            format!("{}.{}", integer_part, decimal_part)
        }
    }

    /// Get the deterministic Smart Account address for this signer
    /// using SimpleAccountFactory
    pub async fn get_smart_account_address(
        &self,
        factory_address: Address,
        provider: std::sync::Arc<AmoyProvider>,
    ) -> Result<Address, Box<dyn std::error::Error + Send + Sync>> {
        // Salt = 0 for default account
        let salt = U256::zero();
        crate::wallet::aa::get_smart_account_address(factory_address, self.address, salt, provider).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_wallet() {
        let wallet = UserWallet::create_new().unwrap();
        // Address should be 42 chars (0x + 40 hex chars)
        assert_eq!(wallet.address_string().len(), 42);
    }

    #[test]
    fn test_restore_wallet() {
        let wallet1 = UserWallet::create_new().unwrap();
        let key_bytes = wallet1.private_key_bytes();
        
        let wallet2 = UserWallet::from_private_key(&key_bytes).unwrap();
        assert_eq!(wallet1.address, wallet2.address);
    }

    #[test]
    fn test_format_balance() {
        // 1 MATIC = 10^18 wei
        let one_matic = U256::from(1_000_000_000_000_000_000u64);
        let formatted = UserWallet::format_balance(one_matic);
        assert!(formatted.starts_with("1."));
    }
}
