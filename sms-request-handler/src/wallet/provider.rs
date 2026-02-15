use ethers::providers::{Http, Middleware, Provider};
use std::sync::Arc;

use super::chains::{Chain, MultiChainProvider};

/// Polygon Amoy testnet chain ID (deprecated, use Chain::PolygonAmoy.chain_id())
pub const POLYGON_AMOY_CHAIN_ID: u64 = 80002;

/// Polygon Amoy RPC URL (deprecated, use Chain::PolygonAmoy.rpc_url())
pub const POLYGON_AMOY_RPC: &str = "https://rpc-amoy.polygon.technology";

/// Provider type for Polygon Amoy (kept for backward compatibility)
pub type AmoyProvider = Provider<Http>;

/// Create a provider for Polygon Amoy testnet (legacy)
pub fn create_amoy_provider() -> AmoyProvider {
    Provider::<Http>::try_from(POLYGON_AMOY_RPC).expect("Invalid RPC URL")
}

/// Shared provider wrapped in Arc for thread-safe access (legacy)
pub fn create_shared_provider() -> Arc<AmoyProvider> {
    Arc::new(create_amoy_provider())
}

/// Create a new multi-chain provider with all testnets
pub fn create_multi_chain_provider() -> MultiChainProvider {
    MultiChainProvider::new()
}

/// Create a provider for a specific chain
pub fn create_chain_provider(chain: Chain) -> Arc<Provider<Http>> {
    Arc::new(Provider::<Http>::try_from(chain.rpc_url()).expect("Invalid RPC URL"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_connection() {
        let provider = create_amoy_provider();
        let chain_id = provider.get_chainid().await;
        // May fail if no network, that's ok for unit test
        if let Ok(id) = chain_id {
            assert_eq!(id.as_u64(), POLYGON_AMOY_CHAIN_ID);
        }
    }

    #[test]
    fn test_multi_chain_provider_creation() {
        let provider = create_multi_chain_provider();
        // Should have all testnets by default
        assert!(provider.get(Chain::PolygonAmoy).is_some());
        assert!(provider.get(Chain::BaseSepolia).is_some());
    }
}

