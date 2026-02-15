use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Form,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::commands::CommandProcessor;
use crate::sms::TwilioClient;

/// Incoming SMS webhook payload from Twilio
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IncomingSms {
    /// The phone number that sent the message
    pub from: String,
    /// The phone number the message was sent to (your Twilio number)
    #[serde(default)]
    pub to: String,
    /// The body of the SMS message
    pub body: String,
    /// Twilio's unique ID for this message
    #[serde(default)]
    pub message_sid: String,
    /// Number of media items attached (MMS)
    #[serde(default)]
    pub num_media: String,
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub twilio: Arc<TwilioClient>,
    pub command_processor: Arc<CommandProcessor>,
}

/// TwiML response for Twilio
struct TwimlResponse(String);

impl IntoResponse for TwimlResponse {
    fn into_response(self) -> Response {
        (
            StatusCode::OK,
            [("Content-Type", "text/xml")],
            self.0,
        )
            .into_response()
    }
}

/// JSON response for SMSCountry
struct JsonResponse(String);

impl IntoResponse for JsonResponse {
    fn into_response(self) -> Response {
        (
            StatusCode::OK,
            [("Content-Type", "application/json")],
            self.0,
        )
            .into_response()
    }
}

/// Handler for incoming SMS messages from Twilio (Form-encoded)
///
/// Responds immediately with empty TwiML to avoid Twilio's 15s timeout,
/// then processes the command and sends the reply via Twilio REST API.
pub async fn incoming_sms_handler(
    State(state): State<AppState>,
    Form(sms): Form<IncomingSms>,
) -> impl IntoResponse {
    tracing::info!(
        from = %sms.from,
        body = %sms.body,
        "Received SMS (Twilio format)"
    );

    let from = sms.from.clone();
    let body = sms.body.clone();
    let processor = state.command_processor.clone();
    let twilio = state.twilio.clone();

    // Process command synchronously for testing/verification
    let response_text = processor.process(&from, &body).await;

    tracing::info!(
        to = %from,
        response = %response_text,
        "Returning SMS response via webhook"
    );

    let twiml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><Response><Message>{}</Message></Response>"#,
        // Basic XML escaping
        escape_xml(&response_text)
    );

    TwimlResponse(twiml)
}

/// Handler for incoming SMS messages from SMSCountry (JSON format)
pub async fn incoming_sms_json_handler(
    State(state): State<AppState>,
    axum::extract::Json(sms): axum::extract::Json<IncomingSms>,
) -> impl IntoResponse {
    tracing::info!(
        from = %sms.from,
        body = %sms.body,
        "Received SMS (JSON format)"
    );

    // Process the command
    let response_text = state
        .command_processor
        .process(&sms.from, &sms.body)
        .await;

    tracing::info!(
        to = %sms.from,
        response = %response_text,
        "Sending SMS response"
    );

    // Return JSON response
    let json_response = serde_json::json!({
        "success": true,
        "response": response_text
    });

    JsonResponse(json_response.to_string())
}


/// Escape special XML characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("Hello & Goodbye"), "Hello &amp; Goodbye");
        assert_eq!(escape_xml("<script>"), "&lt;script&gt;");
    }
}
