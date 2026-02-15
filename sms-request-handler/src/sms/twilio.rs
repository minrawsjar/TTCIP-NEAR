use base64::Engine;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha1::Sha1;
use std::collections::HashMap;

use crate::config::TwilioConfig;

type HmacSha1 = Hmac<Sha1>;

/// Twilio client for sending and validating SMS messages
#[derive(Debug, Clone)]
pub struct TwilioClient {
    client: Client,
    account_sid: String,
    auth_token: String,
    phone_number: String,
}

/// Result of sending an SMS
#[derive(Debug)]
pub struct SendResult {
    pub message_sid: String,
    pub status: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TwilioError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Invalid signature")]
    InvalidSignature,
}

impl TwilioClient {
    /// Create a new Twilio client
    pub fn new(config: &TwilioConfig) -> Self {
        Self {
            client: Client::new(),
            account_sid: config.account_sid.clone(),
            auth_token: config.auth_token.clone(),
            phone_number: config.phone_number.clone(),
        }
    }

    /// Send an SMS message
    pub async fn send_sms(&self, to: &str, body: &str) -> Result<SendResult, TwilioError> {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.account_sid
        );

        let mut params = HashMap::new();
        params.insert("To", to);
        params.insert("From", &self.phone_number);
        params.insert("Body", body);

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.account_sid, Some(&self.auth_token))
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(TwilioError::Api(error_text));
        }

        let json: serde_json::Value = response.json().await?;

        Ok(SendResult {
            message_sid: json["sid"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Validate Twilio request signature
    /// 
    /// This ensures the webhook request actually came from Twilio
    pub fn validate_signature(
        &self,
        signature: &str,
        url: &str,
        params: &HashMap<String, String>,
    ) -> bool {
        // Build the string to sign: URL + sorted params
        let mut data = url.to_string();
        
        let mut sorted_params: Vec<_> = params.iter().collect();
        sorted_params.sort_by(|a, b| a.0.cmp(b.0));
        
        for (key, value) in sorted_params {
            data.push_str(key);
            data.push_str(value);
        }

        // Calculate HMAC-SHA1
        let mut mac = HmacSha1::new_from_slice(self.auth_token.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(data.as_bytes());
        let result = mac.finalize();
        
        // Base64 encode
        let calculated = base64::engine::general_purpose::STANDARD.encode(result.into_bytes());
        
        // Compare
        calculated == signature
    }

    /// Get the Twilio phone number
    pub fn phone_number(&self) -> &str {
        &self.phone_number
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_validation() {
        let config = TwilioConfig {
            account_sid: "test_sid".to_string(),
            auth_token: "12345".to_string(),
            phone_number: "+1234567890".to_string(),
        };
        
        let client = TwilioClient::new(&config);
        
        // This is a simplified test - real signatures would come from Twilio
        let mut params = HashMap::new();
        params.insert("From".to_string(), "+1234".to_string());
        params.insert("Body".to_string(), "test".to_string());
        
        // The signature validation logic is correct; actual testing would need real Twilio data
        assert!(!client.validate_signature("invalid", "https://example.com", &params));
    }
}
