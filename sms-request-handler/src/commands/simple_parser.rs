use std::sync::Arc;
use regex::Regex;
use crate::near_integration::NearWalletService;

/// Parsed SMS command
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Show help/available commands
    Help,
    /// Register a new user with optional wallet name
    Join { ens_name: Option<String> },
    /// Check account balance
    Balance,
    /// Send tokens to someone
    Send { amount: String, token: String, recipient: String },
    /// Buy tokens with airtime
    Buy { amount: String },
    /// Get deposit address
    Deposit,
    /// Redeem voucher
    Redeem { code: String },
    /// Swap tokens
    Swap { amount: String, token: String },
    /// Cash out tokens
    Cashout { amount: String, token: String },
    /// Bridge tokens between chains
    Bridge { amount: String, token: String, from_chain: String, to_chain: String },
    /// Save a contact: SAVE <name> <phone>
    Save { name: String, phone: String },
    /// List contacts
    Contacts,
    /// Switch chain: CHAIN <name>
    SwitchChain { chain: String },
    /// Unknown command
    Unknown(String),
}

/// Simple command processing engine for NEAR wallet system
#[derive(Clone)]
pub struct SimpleCommandProcessor {
    pub near_wallet: Arc<NearWalletService>,
}

impl SimpleCommandProcessor {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize NEAR wallet service
        let near_wallet = Arc::new(NearWalletService::new()?);
        
        Ok(Self { 
            near_wallet,
        })
    }

    /// Parse SMS text into a command
    pub fn parse(&self, text: &str) -> Command {
        let text = text.trim().to_uppercase();
        
        // Handle JOIN commands first (most common)
        if text.starts_with("JOIN") {
            let parts: Vec<&str> = text.splitn(2, ' ').collect();
            if parts.len() > 1 && !parts[1].is_empty() {
                return Command::Join { ens_name: Some(parts[1].to_string()) };
            }
            return Command::Join { ens_name: None };
        }

        // Handle other commands
        match text.as_str() {
            "HELP" | "MENU" | "?" => Command::Help,
            "BALANCE" | "BAL" => Command::Balance,
            "DEPOSIT" | "DEP" => Command::Deposit,
            "CONTACTS" => Command::Contacts,
            _ => {
                // Try to parse SEND command: SEND 10 TXTC TO alice
                if text.starts_with("SEND") {
                    if let Some(captures) = Regex::new(r#"SEND\s+(\d+(?:\.\d+)?)\s+(\w+)\s+TO\s+(.+)"#)
                        .unwrap().captures(&text) {
                        return Command::Send {
                            amount: captures[1].to_string(),
                            token: captures[2].to_string(),
                            recipient: captures[3].to_string(),
                        };
                    }
                }
                
                // Try to parse SWAP command: SWAP 10 TXTC FOR NEAR
                if text.starts_with("SWAP") {
                    if let Some(captures) = Regex::new(r#"SWAP\s+(\d+(?:\.\d+)?)\s+(\w+)\s+FOR\s+(\w+)"#)
                        .unwrap().captures(&text) {
                        return Command::Swap {
                            amount: captures[1].to_string(),
                            token: captures[2].to_string(),
                        };
                    }
                }
                
                // Try to parse CASHOUT command: CASHOUT 10 TXTC
                if text.starts_with("CASHOUT") {
                    if let Some(captures) = Regex::new(r#"CASHOUT\s+(\d+(?:\.\d+)?)\s+(\w+)"#)
                        .unwrap().captures(&text) {
                        return Command::Cashout {
                            amount: captures[1].to_string(),
                            token: captures[2].to_string(),
                        };
                    }
                }
                
                // Try to parse BUY command: BUY 10
                if text.starts_with("BUY") {
                    if let Some(captures) = Regex::new(r#"BUY\s+(\d+(?:\.\d+)?)"#)
                        .unwrap().captures(&text) {
                        return Command::Buy { amount: captures[1].to_string() };
                    }
                }
                
                // Try to parse REDEEM command: REDEEM CODE123
                if text.starts_with("REDEEM") {
                    if let Some(captures) = Regex::new(r#"REDEEM\s+(\w+)"#)
                        .unwrap().captures(&text) {
                        return Command::Redeem { code: captures[1].to_string() };
                    }
                }

                Command::Unknown(text.to_string())
            }
        }
    }

    /// Process an incoming SMS and return the response
    pub async fn process(&self, from: &str, body: &str) -> String {
        let command = self.parse(body);
        
        tracing::debug!(
            from = %from,
            command = ?command,
            "Processing command"
        );

        match command {
            Command::Help => self.help_response(),
            Command::Join { ens_name } => self.join_response(from, ens_name).await,
            Command::Balance => self.balance_response(from).await,
            Command::Send { amount, token, recipient } => self.send_response(from, &amount, &token, &recipient).await,
            Command::Buy { amount } => self.buy_response(from, &amount).await,
            Command::Deposit => self.deposit_response(from).await,
            Command::Redeem { code } => self.redeem_response(from, &code).await,
            Command::Swap { amount, token } => self.swap_response(from, &amount, &token).await,
            Command::Cashout { amount, token } => self.cashout_response(from, &amount, &token).await,
            Command::Bridge { amount, token, from_chain, to_chain } => self.bridge_response(from, &amount, &token, &from_chain, &to_chain).await,
            Command::Save { name, phone } => self.save_response(from, &name, &phone).await,
            Command::Contacts => self.contacts_response(from).await,
            Command::SwitchChain { chain } => self.chain_response(from, &chain).await,
            Command::Unknown(text) => self.unknown_response(&text),
        }
    }

    fn help_response(&self) -> String {
        format!(
            "📱 Text-to-Chain NEAR Commands\n\n🏷️  Account Management:\n• JOIN alice - Create wallet (alice.testnet)\n• BALANCE - Check NEAR & TXTC balance\n• DEPOSIT - Get deposit address\n\n💰 Send & Swap:\n• SEND 1 NEAR TO bob.testnet - Send NEAR\n• SEND 100 TXTC TO bob.testnet - Send TXTC\n• SWAP 100 TXTC FOR NEAR - Exchange tokens\n• SWAP 0.1 NEAR FOR TXTC - Get TXTC\n\n🏧 Cashout:\n• CASHOUT 0.5 NEAR - Convert to USDC\n\n📞 Contacts:\n• SAVE alice +1234567890 - Save contact\n• CONTACTS - List saved contacts\n\n❓ Help:\n• MENU - Show this menu\n• HELP - Show commands"
        )
    }

    async fn join_response(&self, from: &str, ens_name: Option<String>) -> String {
        match self.near_wallet.create_wallet(from, ens_name).await {
            Ok(account_name) => format!(
                "🎉 NEAR Wallet Created!\n\n📱 Phone: {}\n🏷️  Account: {}\n\n💰 Fund your wallet with NEAR to start using Text-to-Chain.\n\nCommands: BALANCE, DEPOSIT, SEND, SWAP, MENU",
                from, account_name
            ),
            Err(e) => format!("❌ Failed to create wallet: {}\n\nTry JOIN <name> or ensure backend is running.", e),
        }
    }

    async fn balance_response(&self, from: &str) -> String {
        match self.near_wallet.get_balances(from).await {
            Ok((near_balance, txtc_balance)) => {
                let near_formatted: f64 = near_balance.parse().unwrap_or(0.0);
                let txtc_formatted: f64 = txtc_balance.parse().unwrap_or(0.0);
                let account = self.near_wallet.phone_to_account_name(from).await;
                
                format!(
                    "💰 NEAR Wallet Balance\n\n🏷️  Account: {}\n💎 NEAR: {:.6} NEAR\n🪙 TXTC: {:.6} TXTC\n\n💡 Send NEAR to your account to start trading",
                    account,
                    near_formatted,
                    txtc_formatted
                )
            }
            Err(e) => {
                format!(
                    "❌ Failed to get balance\n\nError: {}\n\nPlease try again later.",
                    e
                )
            }
        }
    }

    async fn send_response(&self, from: &str, amount: &str, token: &str, recipient: &str) -> String {
        let amount_u128 = match amount.parse::<u128>() {
            Ok(a) => near_sdk::json_types::U128(a),
            Err(_) => return format!("❌ Invalid amount: {}", amount),
        };

        if token.to_uppercase() == "NEAR" {
            match self.near_wallet.send_near(from, recipient, amount_u128).await {
                Ok(tx_hash) => {
                    format!(
                        "✅ NEAR Transfer Initiated!\n\n📤 From: {}\n📥 To: {}\n💎 Amount: {} NEAR\n🔗 TX: {}\n\n💰 Check balance with: BALANCE",
                        from, // using phone here as we don't have account name easily without another call
                        recipient,
                        amount,
                        tx_hash
                    )
                }
                Err(e) => format!("❌ Failed to send NEAR: {}", e)
            }
        } else if token.to_uppercase() == "TXTC" {
            match self.near_wallet.send_txtc(from, recipient, amount_u128).await {
                Ok(tx_hash) => {
                    format!(
                        "✅ TXTC Transfer Initiated!\n\n📤 From: {}\n📥 To: {}\n🪙 Amount: {} TXTC\n🔗 TX: {}\n\n💰 Check balance with: BALANCE",
                        from,
                        recipient,
                        amount,
                        tx_hash
                    )
                }
                Err(e) => format!("❌ Failed to send TXTC: {}", e)
            }
        } else {
            format!("❌ Token {} not supported. Use NEAR or TXTC for transfers.", token)
        }
    }

    async fn buy_response(&self, from: &str, amount: &str) -> String {
        match self.near_wallet.buy_with_airtime(from, amount).await {
            Ok(msg) => format!("✅ {}\n\nReply BALANCE to check.", msg),
            Err(e) => format!("❌ Buy failed: {}", e),
        }
    }

    async fn deposit_response(&self, from: &str) -> String {
        match self.near_wallet.get_deposit_address(from).await {
            Ok(address) => {
                format!(
                    "📥 Deposit Address: {}\n\nSend NEAR to this address to fund your wallet.\n\n💰 Minimum deposit: 0.1 NEAR",
                    address
                )
            }
            Err(e) => {
                format!(
                    "❌ Failed to get deposit address\n\nError: {}\n\nPlease try again.",
                    e
                )
            }
        }
    }

    async fn redeem_response(&self, from: &str, code: &str) -> String {
        let account = self.near_wallet.phone_to_account_name(from).await;
        match self.near_wallet.redeem_voucher(&account, code, from).await {
             Ok((token, eth)) => format!(
                "✅ Voucher redeemed!\n\nReceived:\n{} TXTC\n{} NEAR (gas)\n\nReply BALANCE to check.",
                token, eth
            ),
            Err(e) => format!("❌ Redeem failed: {}", e),
        }
    }

    async fn swap_response(&self, from: &str, amount: &str, token: &str) -> String {
        let amount_u128 = match amount.parse::<u128>() {
            Ok(a) => near_sdk::json_types::U128(a),
            Err(_) => return format!("❌ Invalid amount: {}", amount),
        };

        if token.to_uppercase() == "TXTC" {
            match self.near_wallet.swap_txtc_for_near(from, amount_u128).await {
                Ok(msg) => format!("✅ {}\n\nReply BALANCE to check.", msg),
                Err(e) => format!("❌ Failed to swap TXTC: {}", e)
            }
        } else if token.to_uppercase() == "NEAR" {
            match self.near_wallet.swap_near_for_txtc(from, amount_u128).await {
                Ok(msg) => format!("✅ {}\n\nReply BALANCE to check.", msg),
                Err(e) => format!("❌ Failed to swap NEAR: {}", e)
            }
        } else {
            format!("❌ Cannot swap {}. Use NEAR or TXTC.", token)
        }
    }

    async fn cashout_response(&self, from: &str, amount: &str, token: &str) -> String {
        if token.to_uppercase() == "NEAR" {
             match self.near_wallet.cashout(from, amount).await {
                 Ok(msg) => format!("✅ {}\n\nYou will receive an SMS confirmation shortly.", msg),
                 Err(e) => format!("❌ Cashout failed: {}", e)
             }
        } else {
            format!("❌ Cannot cashout {} directly. Please swap to NEAR first.", token)
        }
    }

    async fn bridge_response(&self, from: &str, amount: &str, token: &str, from_chain: &str, to_chain: &str) -> String {
        format!("🌉 Bridge {} {} from {} to {} - Feature coming soon!", amount, token, from_chain, to_chain)
    }

    async fn save_response(&self, from: &str, name: &str, phone: &str) -> String {
        format!("📝 Saved contact: {} ({}) - Feature coming soon!", name, phone)
    }

    async fn contacts_response(&self, from: &str) -> String {
        format!("📋 Address Book - Feature coming soon!")
    }

    async fn chain_response(&self, from: &str, chain: &str) -> String {
        format!("⛓️ Switched to {} chain - Feature coming soon!", chain)
    }

    fn unknown_response(&self, text: &str) -> String {
        format!("❓ Unknown command: {}\n\nSend MENU for available commands.", text)
    }
}
