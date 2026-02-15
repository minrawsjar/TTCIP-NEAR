use ethers::prelude::*;
use std::sync::Arc;
use super::config::ContractConfig;

// ABI definitions (simplified - use full ABIs in production)
abigen!(
    EntryPointV3,
    r#"[
        function redeemVoucher(string code, address user, bool swapToEth) external returns (uint256 tokenAmount, uint256 ethAmount)
        function swapTokenForEth(address user, uint256 tokenAmount, uint256 minEthOut) external returns (uint256 ethOut)
        function swapEthForToken(address user, uint256 minTokenOut) external payable returns (uint256 tokenOut)
        function getSwapQuote(uint256 amount, bool isTokenToEth) external view returns (uint256)
        event VoucherRedeemed(address indexed user, uint256 tokenAmount, uint256 ethAmount, uint256 gasReserve)
        event TokensSwapped(address indexed user, uint256 amountIn, uint256 amountOut, bool isTokenToEth)
    ]"#
);

abigen!(
    TokenXYZ,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function transfer(address to, uint256 amount) external returns (bool)
        function approve(address spender, uint256 amount) external returns (bool)
        function allowance(address owner, address spender) external view returns (uint256)
    ]"#
);

pub struct ContractService {
    provider: Arc<Provider<Http>>,
    wallet: LocalWallet,
    entry_point: EntryPointV3<SignerMiddleware<Provider<Http>, LocalWallet>>,
    token_xyz: TokenXYZ<SignerMiddleware<Provider<Http>, LocalWallet>>,
}

impl ContractService {
    pub async fn new(config: ContractConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let provider = Provider::<Http>::try_from(&config.rpc_url)?;
        let provider = Arc::new(provider);
        
        let wallet: LocalWallet = config.private_key.parse()?;
        let wallet = wallet.with_chain_id(config.chain_id);
        
        let client = SignerMiddleware::new(provider.clone(), wallet.clone());
        let client = Arc::new(client);
        
        let entry_point = EntryPointV3::new(
            config.contracts.entry_point.parse::<Address>()?,
            client.clone(),
        );
        
        let token_xyz = TokenXYZ::new(
            config.contracts.token_xyz.parse::<Address>()?,
            client.clone(),
        );
        
        Ok(Self {
            provider,
            wallet,
            entry_point,
            token_xyz,
        })
    }
    
    /// Redeem voucher for user
    /// SMS Command: REDEEM <code>
    pub async fn redeem_voucher(
        &self,
        voucher_code: &str,
        user_address: Address,
        auto_swap_to_eth: bool,
    ) -> Result<RedeemResult, Box<dyn std::error::Error>> {
        let tx = self.entry_point
            .redeem_voucher(voucher_code.to_string(), user_address, auto_swap_to_eth)
            .send()
            .await?;
        
        let receipt = tx.await?.ok_or("Transaction failed")?;
        
        // Parse events
        for log in receipt.logs {
            if let Ok(event) = self.entry_point.decode_event::<VoucherRedeemedFilter>(
                "VoucherRedeemed",
                log.topics.clone(),
                log.data.clone(),
            ) {
                return Ok(RedeemResult {
                    success: true,
                    token_amount: format_ether(event.token_amount),
                    eth_amount: format_ether(event.eth_amount),
                    tx_hash: format!("{:?}", receipt.transaction_hash),
                });
            }
        }
        
        Ok(RedeemResult {
            success: true,
            token_amount: "0".to_string(),
            eth_amount: "0".to_string(),
            tx_hash: format!("{:?}", receipt.transaction_hash),
        })
    }
    
    /// Swap tokens for ETH
    /// SMS Command: SWAP <amount> TXTC
    pub async fn swap_token_for_eth(
        &self,
        user_address: Address,
        token_amount: U256,
        min_eth_out: U256,
    ) -> Result<SwapResult, Box<dyn std::error::Error>> {
        let tx = self.entry_point
            .swap_token_for_eth(user_address, token_amount, min_eth_out)
            .send()
            .await?;
        
        let receipt = tx.await?.ok_or("Transaction failed")?;
        
        for log in receipt.logs {
            if let Ok(event) = self.entry_point.decode_event::<TokensSwappedFilter>(
                "TokensSwapped",
                log.topics.clone(),
                log.data.clone(),
            ) {
                return Ok(SwapResult {
                    success: true,
                    amount_out: format_ether(event.amount_out),
                    tx_hash: format!("{:?}", receipt.transaction_hash),
                });
            }
        }
        
        Ok(SwapResult {
            success: true,
            amount_out: "0".to_string(),
            tx_hash: format!("{:?}", receipt.transaction_hash),
        })
    }
    
    /// Get user token balance
    /// SMS Command: BALANCE
    pub async fn get_token_balance(
        &self,
        user_address: Address,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let balance = self.token_xyz.balance_of(user_address).call().await?;
        Ok(format_ether(balance))
    }
    
    /// Get user ETH balance
    pub async fn get_eth_balance(
        &self,
        user_address: Address,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let balance = self.provider.get_balance(user_address, None).await?;
        Ok(format_ether(balance))
    }
    
    /// Get swap quote
    pub async fn get_swap_quote(
        &self,
        amount: U256,
        is_token_to_eth: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let quote = self.entry_point
            .get_swap_quote(amount, is_token_to_eth)
            .call()
            .await?;
        Ok(format_ether(quote))
    }
}

#[derive(Debug, Clone)]
pub struct RedeemResult {
    pub success: bool,
    pub token_amount: String,
    pub eth_amount: String,
    pub tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct SwapResult {
    pub success: bool,
    pub amount_out: String,
    pub tx_hash: String,
}

fn format_ether(value: U256) -> String {
    let eth = ethers::utils::format_ether(value);
    eth
}
