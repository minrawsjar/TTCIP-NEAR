use ethers::providers::{Http, Provider};
use ethers::types::Address;
use std::str::FromStr;
use std::sync::Arc;

/// Supported blockchain networks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chain {
    /// Polygon Amoy Testnet
    PolygonAmoy,
    /// Polygon Mainnet
    PolygonMainnet,
    /// Base Sepolia Testnet
    BaseSepolia,
    /// Base Mainnet
    BaseMainnet,
    /// Ethereum Sepolia Testnet
    EthereumSepolia,
    /// Ethereum Mainnet
    EthereumMainnet,
    /// Arbitrum Sepolia Testnet
    ArbitrumSepolia,
    /// Arbitrum One Mainnet
    ArbitrumOne,
}

impl Chain {
    /// Get chain ID
    pub fn chain_id(&self) -> u64 {
        match self {
            Chain::PolygonAmoy => 80002,
            Chain::PolygonMainnet => 137,
            Chain::BaseSepolia => 84532,
            Chain::BaseMainnet => 8453,
            Chain::EthereumSepolia => 11155111,
            Chain::EthereumMainnet => 1,
            Chain::ArbitrumSepolia => 421614,
            Chain::ArbitrumOne => 42161,
        }
    }

    /// Get RPC URL (public endpoints)
    pub fn rpc_url(&self) -> &'static str {
        match self {
            Chain::PolygonAmoy => "https://rpc-amoy.polygon.technology",
            Chain::PolygonMainnet => "https://polygon-rpc.com",
            Chain::BaseSepolia => "https://sepolia.base.org",
            Chain::BaseMainnet => "https://mainnet.base.org",
            Chain::EthereumSepolia => "https://1rpc.io/sepolia",
            Chain::EthereumMainnet => "https://eth.llamarpc.com",
            Chain::ArbitrumSepolia => "https://sepolia-rollup.arbitrum.io/rpc",
            Chain::ArbitrumOne => "https://arb1.arbitrum.io/rpc",
        }
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Chain::PolygonAmoy => "Polygon Amoy",
            Chain::PolygonMainnet => "Polygon",
            Chain::BaseSepolia => "Base Sepolia",
            Chain::BaseMainnet => "Base",
            Chain::EthereumSepolia => "Ethereum Sepolia",
            Chain::EthereumMainnet => "Ethereum",
            Chain::ArbitrumSepolia => "Arbitrum Sepolia",
            Chain::ArbitrumOne => "Arbitrum",
        }
    }

    /// Get short code for SMS display
    pub fn short_code(&self) -> &'static str {
        match self {
            Chain::PolygonAmoy => "POL-T",
            Chain::PolygonMainnet => "POL",
            Chain::BaseSepolia => "BASE-T",
            Chain::BaseMainnet => "BASE",
            Chain::EthereumSepolia => "ETH-T",
            Chain::EthereumMainnet => "ETH",
            Chain::ArbitrumSepolia => "ARB-T",
            Chain::ArbitrumOne => "ARB",
        }
    }

    /// Get native token symbol
    pub fn native_token(&self) -> &'static str {
        match self {
            Chain::PolygonAmoy | Chain::PolygonMainnet => "MATIC",
            Chain::BaseSepolia | Chain::BaseMainnet => "ETH",
            Chain::EthereumSepolia | Chain::EthereumMainnet => "ETH",
            Chain::ArbitrumSepolia | Chain::ArbitrumOne => "ETH",
        }
    }

    /// Get USDC contract address (None if not deployed)
    pub fn usdc_address(&self) -> Option<Address> {
        let addr_str = match self {
            Chain::PolygonAmoy => "0x41E94Eb019C0762f9Bfcf9Fb1E58725BfB0e7582", // Test USDC
            Chain::PolygonMainnet => "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359",
            Chain::BaseSepolia => "0x036CbD53842c5426634e7929541eC2318f3dCF7e", // Test USDC
            Chain::BaseMainnet => "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            Chain::EthereumSepolia => "0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238", // Test USDC
            Chain::EthereumMainnet => "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            Chain::ArbitrumSepolia => return None, // No official test USDC
            Chain::ArbitrumOne => "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
        };
        Address::from_str(addr_str).ok()
    }

    /// Check if chain is a testnet
    pub fn is_testnet(&self) -> bool {
        matches!(
            self,
            Chain::PolygonAmoy
                | Chain::BaseSepolia
                | Chain::EthereumSepolia
                | Chain::ArbitrumSepolia
        )
    }

    /// Get all supported testnets
    pub fn testnets() -> Vec<Chain> {
        vec![
            Chain::PolygonAmoy,
            Chain::BaseSepolia,
            Chain::EthereumSepolia,
            Chain::ArbitrumSepolia,
        ]
    }

    /// Get all supported mainnets
    pub fn mainnets() -> Vec<Chain> {
        vec![
            Chain::PolygonMainnet,
            Chain::BaseMainnet,
            Chain::EthereumMainnet,
            Chain::ArbitrumOne,
        ]
    }

    /// Parse chain from user input (case-insensitive)
    pub fn from_input(input: &str) -> Option<Chain> {
        match input.to_uppercase().as_str() {
            "POLYGON" | "POL" | "MATIC" => Some(Chain::PolygonMainnet),
            "POLYGON-AMOY" | "POL-T" | "AMOY" => Some(Chain::PolygonAmoy),
            "BASE" => Some(Chain::BaseMainnet),
            "BASE-SEPOLIA" | "BASE-T" => Some(Chain::BaseSepolia),
            "ETH" | "ETHEREUM" => Some(Chain::EthereumMainnet),
            "ETH-SEPOLIA" | "ETH-T" | "SEPOLIA" => Some(Chain::EthereumSepolia),
            "ARB" | "ARBITRUM" => Some(Chain::ArbitrumOne),
            "ARB-SEPOLIA" | "ARB-T" => Some(Chain::ArbitrumSepolia),
            _ => None,
        }
    }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Provider type alias
pub type ChainProvider = Provider<Http>;

/// Chain-specific provider
#[derive(Clone)]
pub struct MultiChainProvider {
    providers: std::collections::HashMap<Chain, Arc<ChainProvider>>,
}

impl MultiChainProvider {
    /// Create a new multi-chain provider with all supported chains
    pub fn new() -> Self {
        let mut providers = std::collections::HashMap::new();

        // Initialize providers for all testnets by default
        for chain in Chain::testnets() {
            if let Ok(provider) = Provider::<Http>::try_from(chain.rpc_url()) {
                providers.insert(chain, Arc::new(provider));
            }
        }

        Self { providers }
    }

    /// Create provider with specific chains
    pub fn with_chains(chains: &[Chain]) -> Self {
        let mut providers = std::collections::HashMap::new();

        for chain in chains {
            if let Ok(provider) = Provider::<Http>::try_from(chain.rpc_url()) {
                providers.insert(*chain, Arc::new(provider));
            }
        }

        Self { providers }
    }

    /// Get provider for a specific chain
    pub fn get(&self, chain: Chain) -> Option<Arc<ChainProvider>> {
        self.providers.get(&chain).cloned()
    }

    /// Get or create provider for a chain
    pub fn get_or_create(&mut self, chain: Chain) -> Arc<ChainProvider> {
        if let Some(provider) = self.providers.get(&chain) {
            return provider.clone();
        }

        let provider = Arc::new(
            Provider::<Http>::try_from(chain.rpc_url()).expect("Invalid RPC URL"),
        );
        self.providers.insert(chain, provider.clone());
        provider
    }

    /// List available chains
    pub fn available_chains(&self) -> Vec<Chain> {
        self.providers.keys().copied().collect()
    }
}

impl Default for MultiChainProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_ids() {
        assert_eq!(Chain::PolygonAmoy.chain_id(), 80002);
        assert_eq!(Chain::BaseMainnet.chain_id(), 8453);
        assert_eq!(Chain::EthereumMainnet.chain_id(), 1);
    }

    #[test]
    fn test_chain_from_input() {
        assert_eq!(Chain::from_input("polygon"), Some(Chain::PolygonMainnet));
        assert_eq!(Chain::from_input("BASE"), Some(Chain::BaseMainnet));
        assert_eq!(Chain::from_input("eth"), Some(Chain::EthereumMainnet));
        assert_eq!(Chain::from_input("unknown"), None);
    }

    #[test]
    fn test_usdc_addresses() {
        assert!(Chain::PolygonMainnet.usdc_address().is_some());
        assert!(Chain::BaseMainnet.usdc_address().is_some());
        assert!(Chain::EthereumMainnet.usdc_address().is_some());
    }

    #[test]
    fn test_multi_chain_provider() {
        let provider = MultiChainProvider::new();
        assert!(provider.get(Chain::PolygonAmoy).is_some());
    }
}
