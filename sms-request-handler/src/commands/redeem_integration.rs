/// STEP 1: REDEEM Command Integration with Smart Contracts
/// 
/// This module shows how to integrate the REDEEM command with the deployed contracts.
/// Replace the existing redeem_response function in parser.rs with this implementation.

use ethers::prelude::*;
use std::sync::Arc;

// Contract ABIs (simplified)
abigen!(
    EntryPointV3,
    r#"[
        function redeemVoucher(string code, address user, bool swapToEth) external returns (uint256 tokenAmount, uint256 ethAmount)
        event VoucherRedeemed(address indexed user, uint256 tokenAmount, uint256 ethAmount, uint256 gasReserve)
    ]"#
);

/// Updated REDEEM command handler with contract integration
pub async fn redeem_response_with_contracts(
    from: &str,
    code: &str,
    user_repo: &crate::db::UserRepository,
    voucher_repo: &crate::db::VoucherRepository,
    deposit_repo: &crate::db::DepositRepository,
    provider: Arc<Provider<Http>>,
    entry_point_address: Address,
    backend_private_key: &str,
) -> String {
    // 1. Check if user has wallet
    let user = match user_repo.find_by_phone(from).await {
        Ok(Some(u)) => u,
        Ok(None) => return "No wallet found.\nReply JOIN to create one.".to_string(),
        Err(_) => return "Database error. Try later.".to_string(),
    };

    // 2. Verify voucher exists in database (before calling contract)
    let voucher = match voucher_repo.get_by_code(code).await {
        Ok(Some(v)) => v,
        Ok(None) => return "❌ Invalid voucher code.".to_string(),
        Err(_) => return "Database error. Try later.".to_string(),
    };

    // Check if already redeemed
    if voucher.redeemed_by.is_some() {
        return "❌ Voucher already used.".to_string();
    }

    // 3. Parse user's wallet address
    let user_address = match user.wallet_address.parse::<Address>() {
        Ok(addr) => addr,
        Err(_) => return "Invalid wallet address.".to_string(),
    };

    // 4. Setup contract connection
    let wallet: LocalWallet = match backend_private_key.parse() {
        Ok(w) => w.with_chain_id(11155111u64), // Sepolia
        Err(_) => return "Backend wallet error.".to_string(),
    };

    let client = SignerMiddleware::new(provider.clone(), wallet);
    let client = Arc::new(client);

    let entry_point = EntryPointV3::new(entry_point_address, client);

    // 5. Call smart contract to redeem voucher
    println!("🔄 Calling contract to redeem voucher {} for user {}", code, user_address);

    match entry_point
        .redeem_voucher(code.to_string(), user_address, true) // auto-swap to ETH
        .send()
        .await
    {
        Ok(pending_tx) => {
            println!("📝 Transaction sent, waiting for confirmation...");
            
            // Wait for transaction receipt
            match pending_tx.await {
                Ok(Some(receipt)) => {
                    println!("✅ Transaction confirmed: {:?}", receipt.transaction_hash);

                    // Parse the VoucherRedeemed event to get amounts
                    let mut token_amount = "0".to_string();
                    let mut eth_amount = "0".to_string();

                    for log in receipt.logs {
                        if let Ok(event) = entry_point.decode_event::<VoucherRedeemedFilter>(
                            "VoucherRedeemed",
                            log.topics.clone(),
                            log.data.clone(),
                        ) {
                            token_amount = ethers::utils::format_ether(event.token_amount);
                            eth_amount = ethers::utils::format_ether(event.eth_amount);
                            break;
                        }
                    }

                    // 6. Mark voucher as redeemed in database
                    if let Err(e) = voucher_repo.redeem(code, from).await {
                        println!("⚠️ Warning: Failed to update voucher in DB: {:?}", e);
                    }

                    // 7. Record deposit in database
                    if let Err(e) = deposit_repo.record_deposit(
                        user.id,
                        &format!("{:?}", receipt.transaction_hash),
                        &eth_amount,
                        "ETH",
                    ).await {
                        println!("⚠️ Warning: Failed to record deposit: {:?}", e);
                    }

                    // 8. Send success response
                    format!(
                        "✅ Voucher redeemed!\n\n\
                         Received: {} ETH\n\
                         TX: {:?}\n\n\
                         Reply BALANCE to check.",
                        eth_amount,
                        receipt.transaction_hash
                    )
                }
                Ok(None) => {
                    "⏳ Transaction pending.\nCheck back in 1 minute.".to_string()
                }
                Err(e) => {
                    println!("❌ Transaction failed: {:?}", e);
                    format!("❌ Redemption failed.\nError: {}", e)
                }
            }
        }
        Err(e) => {
            println!("❌ Contract call failed: {:?}", e);
            
            // Parse error message
            let error_msg = e.to_string();
            if error_msg.contains("invalid code") {
                "❌ Invalid voucher code.".to_string()
            } else if error_msg.contains("already redeemed") {
                "❌ Voucher already used.".to_string()
            } else {
                format!("❌ Redemption failed.\nTry again later.")
            }
        }
    }
}

/// Integration instructions:
/// 
/// 1. Add to parser.rs imports:
///    ```rust
///    mod redeem_integration;
///    use redeem_integration::redeem_response_with_contracts;
///    ```
/// 
/// 2. Update CommandProcessor struct to include:
///    ```rust
///    pub struct CommandProcessor {
///        // ... existing fields ...
///        contract_provider: Arc<Provider<Http>>,
///        entry_point_address: Address,
///        backend_private_key: String,
///    }
///    ```
/// 
/// 3. Replace the redeem_response function call in process() with:
///    ```rust
///    Command::Redeem { code } => {
///        redeem_response_with_contracts(
///            from,
///            &code,
///            &user_repo,
///            &voucher_repo,
///            &deposit_repo,
///            self.contract_provider.clone(),
///            self.entry_point_address,
///            &self.backend_private_key,
///        ).await
///    }
///    ```
/// 
/// 4. Add to Cargo.toml dependencies:
///    ```toml
///    ethers = { version = "2.0", features = ["abigen", "ws"] }
///    ```
