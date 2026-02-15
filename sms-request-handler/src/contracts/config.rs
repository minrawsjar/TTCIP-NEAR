use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfig {
    pub chain_id: u64,
    pub rpc_url: String,
    pub private_key: String,
    pub contracts: ContractAddresses,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAddresses {
    pub token_xyz: String,
    pub voucher_manager: String,
    pub pool_manager: String,
    pub entry_point: String,
    pub uniswap_v3_pool: String,
}

impl ContractConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            chain_id: 11155111, // Sepolia
            rpc_url: std::env::var("RPC_URL")?,
            private_key: std::env::var("PRIVATE_KEY")?,
            contracts: ContractAddresses {
                token_xyz: "0x4d054FB258A260982F0bFab9560340d33D9E698B".to_string(),
                voucher_manager: "0x3094e5820F911f9119D201B9E2DdD4b9cf792990".to_string(),
                pool_manager: "0xd9794c0daC0382c11F6Cf4a8365a8A49690Dcfc8".to_string(),
                entry_point: "0x6b5b8b917f3161aeb72105b988E55910e231d240".to_string(),
                uniswap_v3_pool: "0x54fB26024019504e075B98c2834adEB29E779c7e".to_string(),
            },
        })
    }
}
