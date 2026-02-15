use std::collections::HashMap;
use serde_json::json;
use anyhow::{Result, anyhow};
use tracing::{info, error, debug};
use reqwest::Client;

/// Real NEAR integration for Text-to-Chain via API Server
#[derive(Debug, Clone)]
pub struct NearIntegration {
    /// HTTP Client
    client: Client,
    /// API Server URL
    api_url: String,
}

impl NearIntegration {
    /// Create new NEAR integration
    pub fn new() -> Result<Self> {
        let api_url = std::env::var("NEAR_API_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        
        info!("Initializing NEAR integration with API: {}", api_url);

        Ok(Self {
            client: Client::new(),
            api_url,
        })
    }

    /// Helper to get full API URL
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.api_url, path)
    }

    /// Resolve phone number to NEAR account
    /// Uses the balance endpoint which returns the account ID
    pub async fn resolve_account(&self, phone: &str) -> Result<String> {
        let url = self.url(&format!("/api/balance/{}", phone));
        let res = self.client.get(&url).send().await?;
        
        if !res.status().is_success() {
            return Err(anyhow!("Failed to resolve account: {}", res.status()));
        }

        let body: serde_json::Value = res.json().await?;
        
        if let Some(account) = body.get("nearAccount").and_then(|v| v.as_str()) {
            Ok(account.to_string())
        } else {
            // Fallback: assume user{last6digits}.testnet logic or return error
            // If the user isn't registered, we might not get an account.
            Err(anyhow!("Account not found for phone {}", phone))
        }
    }

    /// Create NEAR wallet for phone number
    pub async fn create_wallet(&self, phone: &str, wallet_name: Option<String>) -> Result<String> {
        info!("Creating wallet for {}", phone);
        let url = self.url("/api/join");
        
        let mut payload = HashMap::new();
        payload.insert("phoneNumber", phone.to_string());
        if let Some(name) = wallet_name {
            payload.insert("name", name);
        }

        let res = self.client.post(&url)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_text = res.text().await?;
            error!("Join failed: {}", err_text);
            return Err(anyhow!("Failed to create wallet: {}", err_text));
        }

        let body: serde_json::Value = res.json().await?;
        let account = body.get("nearAccount")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No account returned"))?;

        Ok(account.to_string())
    }

    /// Get user balances (NEAR + TXTC)
    pub async fn get_balances(&self, phone: &str) -> Result<(String, String)> {
        let url = self.url(&format!("/api/balance/{}", phone));
        let res = self.client.get(&url).send().await?;
        
        if !res.status().is_success() {
             return Ok(("0.0".to_string(), "0.0".to_string()));
        }

        let body: serde_json::Value = res.json().await?;
        let balances = body.get("balances").ok_or_else(|| anyhow!("No balances found"))?;
        
        let near = balances.get("near").and_then(|v| v.as_str()).unwrap_or("0").to_string();
        let txtc = balances.get("txtc").and_then(|v| v.as_str()).unwrap_or("0").to_string();

        Ok((near, txtc))
    }

    /// Send NEAR to another account (via /api/transfer)
    pub async fn send_near(&self, from_phone: &str, to_account: &str, amount: &str) -> Result<String> {
        info!("Sending {} NEAR from {} to {}", amount, from_phone, to_account);
        let url = self.url("/api/transfer");
        
        let payload = json!({
            "fromPhone": from_phone,
            "toPhone": to_account, // API handles phone or account
            "amount": amount
        });

        let res = self.client.post(&url)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
             let err = res.text().await?;
             return Err(anyhow!("Transfer failed: {}", err));
        }
        
        let body: serde_json::Value = res.json().await?;
        Ok(body.get("txHash").and_then(|v| v.as_str()).unwrap_or("transfer_initiated").to_string())
    }

    /// Send TXTC to another account (via /api/send)
    pub async fn send_txtc(&self, from_phone: &str, to_account: &str, amount: &str) -> Result<String> {
        info!("Sending {} TXTC from {} to {}", amount, from_phone, to_account);
        
        // 1. Resolve sender address
        let from_account = self.resolve_account(from_phone).await?;
        
        // 2. Call send endpoint
        let url = self.url("/api/send");
        let payload = json!({
            "fromAddress": from_account,
            "toAddress": to_account,
            "amount": amount
        });

        let res = self.client.post(&url)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
             let err = res.text().await?;
             return Err(anyhow!("Send TXTC failed: {}", err));
        }
        
        let body: serde_json::Value = res.json().await?;
        Ok(body.get("txHash").and_then(|v| v.as_str()).unwrap_or("send_ok").to_string())
    }

    /// Swap TXTC for NEAR (via /api/swap)
    pub async fn swap_txtc_for_near(&self, phone: &str, amount: &str) -> Result<String> {
        info!("Swapping {} TXTC to NEAR for {}", amount, phone);
        let user_address = self.resolve_account(phone).await?;
        
        let url = self.url("/api/swap");
        let payload = json!({
            "userAddress": user_address,
            "tokenAmount": amount,
            "direction": "txtc_to_near"
        });

        let res = self.client.post(&url).json(&payload).send().await?;
        
        if !res.status().is_success() {
            let err = res.text().await?;
            return Err(anyhow!("Swap failed: {}", err));
        }
        
        Ok("swap_initiated".to_string())
    }

    /// Swap NEAR for TXTC (via /api/swap)
    pub async fn swap_near_for_txtc(&self, phone: &str, amount: &str) -> Result<String> {
        info!("Swapping {} NEAR to TXTC for {}", amount, phone);
        let user_address = self.resolve_account(phone).await?;
        
        let url = self.url("/api/swap");
        let payload = json!({
            "userAddress": user_address,
            "nearAmount": amount,
            "direction": "near_to_txtc"
        });

        let res = self.client.post(&url).json(&payload).send().await?;
        
        if !res.status().is_success() {
            let err = res.text().await?;
            return Err(anyhow!("Swap failed: {}", err));
        }

        Ok("swap_initiated".to_string())
    }

    /// Redeem voucher
    pub async fn redeem_voucher(&self, account: &str, code: &str, phone: &str) -> Result<(String, String)> {
        info!("Redeeming voucher {} for {}", code, account);
        let url = self.url("/api/redeem");
        let payload = json!({
            "voucherCode": code,
            "userAddress": account,
            "userPhone": phone
        });

        let res = self.client.post(&url).json(&payload).send().await?;
        
        if !res.status().is_success() {
            let err = res.text().await?;
            return Err(anyhow!("Redeem failed: {}", err));
        }

        let body: serde_json::Value = res.json().await?;
        let token_amount = body.get("tokenAmount").and_then(|v| v.as_str()).unwrap_or("0").to_string();
        let eth_amount = body.get("ethAmount").and_then(|v| v.as_str()).or_else(|| body.get("nearAmount").and_then(|v| v.as_str())).unwrap_or("0").to_string();
        
        Ok((token_amount, eth_amount))
    }

    /// Cashout
    pub async fn cashout(&self, phone: &str, amount: &str) -> Result<String> {
        info!("Cashing out {} NEAR for {}", amount, phone);
        let url = self.url("/api/cashout");
        let payload = json!({
            "phoneNumber": phone,
            "amount": amount,
            "userPhone": phone
        });

        let res = self.client.post(&url).json(&payload).send().await?;
        
        if !res.status().is_success() {
            let err = res.text().await?;
            return Err(anyhow!("Cashout failed: {}", err));
        }
        
        Ok("cashout_initiated".to_string())
    }

    /// Buy airtime (placeholder/future)
    pub async fn buy_with_airtime(&self, phone: &str, amount: &str) -> Result<String> {
         let account = self.resolve_account(phone).await?;
         let url = self.url("/api/buy");
         let payload = json!({
             "userAddress": account,
             "amount": amount,
             "userPhone": phone
         });
         
         let res = self.client.post(&url).json(&payload).send().await?;
         if !res.status().is_success() {
             let err = res.text().await?;
             return Err(anyhow!("Buy failed: {}", err));
         }
         Ok("buy_initiated".to_string())
    }

    /// Get deposit address (just the account ID)
    pub async fn get_deposit_address(&self, phone: &str) -> Result<String> {
        self.resolve_account(phone).await
    }
}

/// NEAR wallet service wrapper
#[derive(Debug)]
pub struct NearWalletService {
    pub near_integration: NearIntegration, 
}

impl NearWalletService {
    pub fn new() -> Result<Self> {
        Ok(Self { 
            near_integration: NearIntegration::new()?
        })
    }

    pub async fn create_wallet(&self, phone: &str, wallet_name: Option<String>) -> Result<String> {
        self.near_integration.create_wallet(phone, wallet_name).await
    }

    pub async fn get_balances(&self, phone: &str) -> Result<(String, String)> {
        self.near_integration.get_balances(phone).await
    }

    pub async fn send_near(&self, from_phone: &str, to_account: &str, amount: near_sdk::json_types::U128) -> Result<String> {
        self.near_integration.send_near(from_phone, to_account, &amount.0.to_string()).await
    }

    pub async fn send_txtc(&self, from_phone: &str, to_account: &str, amount: near_sdk::json_types::U128) -> Result<String> {
        self.near_integration.send_txtc(from_phone, to_account, &amount.0.to_string()).await
    }

    pub async fn swap_txtc_for_near(&self, phone: &str, amount: near_sdk::json_types::U128) -> Result<String> {
        self.near_integration.swap_txtc_for_near(phone, &amount.0.to_string()).await
    }

    pub async fn swap_near_for_txtc(&self, phone: &str, amount: near_sdk::json_types::U128) -> Result<String> {
        self.near_integration.swap_near_for_txtc(phone, &amount.0.to_string()).await
    }

    pub async fn get_deposit_address(&self, phone: &str) -> Result<String> {
        self.near_integration.get_deposit_address(phone).await
    }
    
    pub async fn phone_to_account_name(&self, phone: &str) -> String {
        self.near_integration.resolve_account(phone).await.unwrap_or_else(|_| "unknown".to_string())
    }
    
    pub async fn redeem_voucher(&self, account: &str, code: &str, phone: &str) -> Result<(String, String)> {
        self.near_integration.redeem_voucher(account, code, phone).await
    }
    
    pub async fn cashout(&self, phone: &str, amount: &str) -> Result<String> {
        self.near_integration.cashout(phone, amount).await
    }

    pub async fn buy_with_airtime(&self, phone: &str, amount: &str) -> Result<String> {
        self.near_integration.buy_with_airtime(phone, amount).await
    }
}
