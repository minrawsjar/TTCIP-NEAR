use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub twilio: TwilioConfig,
    pub msg91: Msg91Config,
    pub server: ServerConfig,
    pub aa: AaConfig,
    pub admin_private_key: String,
}

#[derive(Debug, Clone)]
pub struct TwilioConfig {
    pub account_sid: String,
    pub auth_token: String,
    pub phone_number: String,
}

#[derive(Debug, Clone)]
pub struct Msg91Config {
    pub auth_key: String,
    pub sender_id: String,
    pub template_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct AaConfig {
    pub bundler_url: String,
    pub entry_point_address: String,
    pub simple_account_factory_address: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        Ok(Config {
            twilio: TwilioConfig {
                account_sid: env::var("TWILIO_ACCOUNT_SID")
                    .map_err(|_| ConfigError::Missing("TWILIO_ACCOUNT_SID"))?,
                auth_token: env::var("TWILIO_AUTH_TOKEN")
                    .map_err(|_| ConfigError::Missing("TWILIO_AUTH_TOKEN"))?,
                phone_number: env::var("TWILIO_PHONE_NUMBER")
                    .map_err(|_| ConfigError::Missing("TWILIO_PHONE_NUMBER"))?,
            },
            msg91: Msg91Config {
                auth_key: env::var("MSG91_AUTH_KEY")
                    .map_err(|_| ConfigError::Missing("MSG91_AUTH_KEY"))?,
                sender_id: env::var("MSG91_SENDER_ID")
                    .unwrap_or_else(|_| "TXTCHAIN".to_string()),
                template_id: env::var("MSG91_TEMPLATE_ID").ok(),
            },
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "3000".to_string())
                    .parse()
                    .map_err(|_| ConfigError::Invalid("SERVER_PORT"))?,
            },
            aa: AaConfig {
                bundler_url: env::var("BUNDLER_URL").unwrap_or_else(|_| "".to_string()),
                entry_point_address: env::var("ENTRY_POINT_ADDRESS").unwrap_or_else(|_| "".to_string()),
                simple_account_factory_address: env::var("SIMPLE_ACCOUNT_FACTORY_ADDRESS").unwrap_or_else(|_| "".to_string()),
            },
            admin_private_key: env::var("ADMIN_PRIVATE_KEY").unwrap_or_else(|_| "".to_string()),
        })
    }

    /// Get server bind address
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    Missing(&'static str),
    #[error("Invalid value for: {0}")]
    Invalid(&'static str),
}
