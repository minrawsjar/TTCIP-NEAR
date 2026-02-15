use ethers::prelude::*;
use ethers::contract::abigen;
use super::chains::{Chain, ChainProvider};
use std::sync::Arc;

// Generate ERC20 contract bindings for USDC
abigen!(
    IERC20,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function decimals() external view returns (uint8)
        function symbol() external view returns (string)
        function transfer(address to, uint256 amount) external returns (bool)
        function approve(address spender, uint256 amount) external returns (bool)
    ]"#
);

/// Token balance information
#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub chain: Chain,
    pub symbol: String,
    pub balance: U256,
    pub decimals: u8,
}

impl TokenBalance {
    /// Format balance as human-readable string
    pub fn formatted(&self) -> String {
        format_token_balance(self.balance, self.decimals)
    }
}

/// Format token balance with proper decimals
pub fn format_token_balance(balance: U256, decimals: u8) -> String {
    if balance.is_zero() {
        return "0.00".to_string();
    }

    let divisor = U256::from(10u64).pow(U256::from(decimals));
    let integer_part = balance / divisor;
    let remainder = balance % divisor;
    
    // Format remainder with leading zeros - U256 to_string doesn't pad
    let remainder_str = remainder.to_string();
    let padded = format!("{:0>width$}", remainder_str, width = decimals as usize);
    let decimal_part = &padded[..std::cmp::min(6, decimals as usize)];
    
    format!("{}.{}", integer_part, decimal_part)
}

/// Get USDC balance for an address on a specific chain
pub async fn get_usdc_balance(
    provider: Arc<ChainProvider>,
    chain: Chain,
    address: Address,
) -> Result<TokenBalance, String> {
    let usdc_address = chain
        .usdc_address()
        .ok_or_else(|| format!("USDC not available on {}", chain.name()))?;

    let contract = IERC20::new(usdc_address, provider);

    let balance = contract
        .balance_of(address)
        .call()
        .await
        .map_err(|e| format!("Failed to get balance: {}", e))?;

    // USDC has 6 decimals on all chains
    Ok(TokenBalance {
        chain,
        symbol: "USDC".to_string(),
        balance,
        decimals: 6,
    })
}

/// Get native token balance (ETH/MATIC)
pub async fn get_native_balance(
    provider: Arc<ChainProvider>,
    chain: Chain,
    address: Address,
) -> Result<TokenBalance, String> {
    let balance = provider
        .get_balance(address, None)
        .await
        .map_err(|e| format!("Failed to get balance: {}", e))?;

    Ok(TokenBalance {
        chain,
        symbol: chain.native_token().to_string(),
        balance,
        decimals: 18,
    })
}

/// All balances for a user on a specific chain
#[derive(Debug, Clone)]
pub struct ChainBalances {
    pub chain: Chain,
    pub native: TokenBalance,
    pub usdc: Option<TokenBalance>,
}

impl ChainBalances {
    /// Format for SMS display (compact)
    pub fn to_sms_string(&self) -> String {
        let native = format!("{} {}", self.native.formatted(), self.native.symbol);
        
        match &self.usdc {
            Some(usdc) => format!(
                "{}: {} | {} USDC",
                self.chain.short_code(),
                native,
                usdc.formatted()
            ),
            None => format!("{}: {}", self.chain.short_code(), native),
        }
    }
}

/// Get all balances for an address on a chain
pub async fn get_chain_balances(
    provider: Arc<ChainProvider>,
    chain: Chain,
    address: Address,
) -> Result<ChainBalances, String> {
    let native = get_native_balance(provider.clone(), chain, address).await?;
    
    let usdc = if chain.usdc_address().is_some() {
        get_usdc_balance(provider, chain, address).await.ok()
    } else {
        None
    };

    Ok(ChainBalances { chain, native, usdc })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_balance() {
        // 1 USDC (6 decimals) = 1_000_000
        let one_usdc = U256::from(1_000_000u64);
        assert_eq!(format_token_balance(one_usdc, 6), "1.000000");

        // 0.5 USDC
        let half_usdc = U256::from(500_000u64);
        assert_eq!(format_token_balance(half_usdc, 6), "0.500000");

        // 1 ETH (18 decimals)
        let one_eth = U256::from(1_000_000_000_000_000_000u64);
        assert_eq!(format_token_balance(one_eth, 18), "1.000000");
    }

    #[test]
    fn test_chain_balances_format() {
        let balances = ChainBalances {
            chain: Chain::PolygonAmoy,
            native: TokenBalance {
                chain: Chain::PolygonAmoy,
                symbol: "MATIC".to_string(),
                balance: U256::from(1_500_000_000_000_000_000u64), // 1.5 MATIC
                decimals: 18,
            },
            usdc: Some(TokenBalance {
                chain: Chain::PolygonAmoy,
                symbol: "USDC".to_string(),
                balance: U256::from(25_500_000u64), // 25.5 USDC
                decimals: 6,
            }),
        };

        let sms = balances.to_sms_string();
        assert!(sms.contains("POL-T"));
        assert!(sms.contains("MATIC"));
        assert!(sms.contains("USDC"));
    }
}
