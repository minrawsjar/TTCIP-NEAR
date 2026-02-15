use ethers::prelude::*;
use ethers::types::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};

/// ERC-4337 UserOperation (v0.6.0 compatible for broadest support)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserOperation {
    pub sender: Address,
    pub nonce: U256,
    pub init_code: Bytes,
    pub call_data: Bytes,
    pub call_gas_limit: U256,
    pub verification_gas_limit: U256,
    pub pre_verification_gas: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    pub paymaster_and_data: Bytes,
    pub signature: Bytes,
}

impl UserOperation {
    /// Pack the UserOperation for signing (hash calculation)
    /// Keccak256(
    ///     sender,
    ///     nonce,
    ///     keccak256(initCode),
    ///     keccak256(callData),
    ///     callGasLimit,
    ///     verificationGasLimit,
    ///     preVerificationGas,
    ///     maxFeePerGas,
    ///     maxPriorityFeePerGas,
    ///     keccak256(paymasterAndData)
    /// )
    pub fn pack(&self) -> Vec<u8> {
        let mut packed: Vec<u8> = Vec::new();
        
        let sender_tokens = ethers::abi::Token::Address(self.sender);
        let nonce_tokens = ethers::abi::Token::Uint(self.nonce);
        let init_code_hash = ethers::abi::Token::FixedBytes(ethers::utils::keccak256(&self.init_code).to_vec());
        let call_data_hash = ethers::abi::Token::FixedBytes(ethers::utils::keccak256(&self.call_data).to_vec());
        let call_gas = ethers::abi::Token::Uint(self.call_gas_limit);
        let ver_gas = ethers::abi::Token::Uint(self.verification_gas_limit);
        let pre_gas = ethers::abi::Token::Uint(self.pre_verification_gas);
        let max_fee = ethers::abi::Token::Uint(self.max_fee_per_gas);
        let max_prio = ethers::abi::Token::Uint(self.max_priority_fee_per_gas);
        let paymaster_hash = ethers::abi::Token::FixedBytes(ethers::utils::keccak256(&self.paymaster_and_data).to_vec());

        ethers::abi::encode(&[
            sender_tokens,
            nonce_tokens,
            init_code_hash,
            call_data_hash,
            call_gas,
            ver_gas,
            pre_gas,
            max_fee,
            max_prio,
            paymaster_hash,
        ])
    }

    /// Calculate the UserOp hash (requestId) to sign
    pub fn hash(&self, entry_point_address: Address, chain_id: u64) -> [u8; 32] {
        let packed = self.pack();
        let user_op_hash = ethers::utils::keccak256(packed);
        
        let enc = ethers::abi::encode(&[
            ethers::abi::Token::FixedBytes(user_op_hash.to_vec()),
            ethers::abi::Token::Address(entry_point_address),
            ethers::abi::Token::Uint(U256::from(chain_id)),
        ]);
        
        ethers::utils::keccak256(enc)
    }
}

// Simple Account Factory ABI (createAccount)
// Simple Account Factory ABI (createAccount)
abigen!(
    SimpleAccountFactory,
    r#"[
        function createAccount(address owner, uint256 salt) public returns (address ret)
        function getAddress(address owner, uint256 salt) public view returns (address)
    ]"#;

    EntryPoint,
    r#"[
         function getNonce(address sender, uint192 key) public view returns (uint256 nonce)
    ]"#;

    SimpleAccount,
    r#"[
        function execute(address dest, uint256 value, bytes calldata func) external
    ]"#
);

/// Get the nonce for a Smart Account from the EntryPoint
pub async fn get_account_nonce(
    entry_point_address: Address,
    sender: Address,
    provider: std::sync::Arc<Provider<Http>>,
) -> Result<U256, Box<dyn std::error::Error + Send + Sync>> {
    let entry_point = EntryPoint::new(entry_point_address, provider);
    // Key is usually 0
    let nonce = entry_point.get_nonce(sender, U256::zero()).call().await?;
    Ok(nonce)
}

/// Helper to get the deterministic address for a user's Smart Account
pub async fn get_smart_account_address(
    factory_address: Address,
    owner_eoa: Address,
    salt: U256,
    provider: std::sync::Arc<Provider<Http>>,
) -> Result<Address, Box<dyn std::error::Error + Send + Sync>> {
    let factory = SimpleAccountFactory::new(factory_address, provider);
    
    // Use getAddress to find the counterfactual address without deploying
    let address = factory.get_address(owner_eoa, salt).call().await?;
    
    Ok(address)
}

/// Client to interact with an ERC-4337 Bundler
#[derive(Clone)]
pub struct BundlerClient {
    client: reqwest::Client,
    bundler_url: String,
}

impl BundlerClient {
    pub fn new(bundler_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            bundler_url,
        }
    }

    /// Send a UserOperation to the bundler
    pub async fn send_user_op(
        &self,
        user_op: UserOperation,
        entry_point: Address,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_sendUserOperation",
            "params": [
                user_op,
                entry_point
            ]
        });

        let response = self.client
            .post(&self.bundler_url)
            .json(&payload)
            .send()
            .await?;

        let body: serde_json::Value = response.json().await?;
        
        if let Some(error) = body.get("error") {
            return Err(format!("Bundler error: {}", error).into());
        }

        if let Some(result) = body.get("result") {
            Ok(result.as_str().unwrap_or("").to_string())
        } else {
            Err("Empty result from bundler".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_user_op_packing() {
        // Test vector from a known valid UserOp (or just structural consistency)
        let op = UserOperation {
            sender: Address::from_str("0x1111111111111111111111111111111111111111").unwrap(),
            nonce: U256::from(1),
            init_code: Bytes::from(vec![0x12, 0x34]),
            call_data: Bytes::from(vec![0x56, 0x78]),
            call_gas_limit: U256::from(100000),
            verification_gas_limit: U256::from(200000),
            pre_verification_gas: U256::from(30000),
            max_fee_per_gas: U256::from(1000000000),
            max_priority_fee_per_gas: U256::from(100000000),
            paymaster_and_data: Bytes::from(vec![]),
            signature: Bytes::from(vec![0xaa, 0xbb]),
        };

        let packed = op.pack();
        assert!(!packed.is_empty());
        
        // Ensure it produces 32-byte hash
        let hash = ethers::utils::keccak256(packed);
        assert_eq!(hash.len(), 32);
    }
}
