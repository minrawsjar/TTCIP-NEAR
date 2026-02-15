use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crate::admin::{admin_routes, AdminState};
use crate::admin_wallet::admin_wallet_routes;
use crate::commands::CommandProcessor;
use crate::db::VoucherRepository;
use crate::sms::{incoming_sms_handler, incoming_sms_json_handler, TwilioClient};
use crate::sms::webhook::AppState;
use sqlx::PgPool;

/// Build the application router with all routes
pub fn create_router(twilio: TwilioClient, command_processor: CommandProcessor) -> Router {
    let state = AppState {
        twilio: Arc::new(twilio),
        command_processor: Arc::new(command_processor),
    };

    Router::new()
        // SMS webhook endpoint - Twilio sends incoming messages here (form-encoded)
        .route("/sms/incoming", post(incoming_sms_handler))
        // SMS webhook endpoint - SMSCountry/generic JSON webhooks
        .route("/webhook/sms", post(incoming_sms_json_handler))
        // Health check endpoint
        .route("/health", get(health_check))
        // Ready check endpoint
        .route("/ready", get(ready_check))
        // Add tracing middleware
        .layer(TraceLayer::new_for_http())
        // Add shared state
        .with_state(state)

}

/// Build router with admin routes (requires voucher repo and db pool)
pub fn create_router_with_admin(
    twilio: TwilioClient, 
    command_processor: CommandProcessor,
    voucher_repo: VoucherRepository,
    admin_token: String,
    db_pool: PgPool,
) -> Router {
    let sms_state = AppState {
        twilio: Arc::new(twilio),
        command_processor: Arc::new(command_processor),
    };

    let admin_state = AdminState {
        voucher_repo: Arc::new(voucher_repo),
        admin_token,
    };

    // Create SMS routes with their state
    let sms_routes = Router::new()
        .route("/sms/incoming", post(incoming_sms_handler))
        .route("/webhook/sms", post(incoming_sms_json_handler))
        .with_state(sms_state);


    // Create admin routes with their state
    let admin_router = admin_routes(admin_state)
        .merge(admin_wallet_routes(Arc::new(db_pool.clone())));
    
    // Merge all routes together
    Router::new()
        .merge(sms_routes)
        .nest("/admin", admin_router)
        .route("/health", get(health_check))
        .route("/ready", get(ready_check))
        .layer(TraceLayer::new_for_http())
}

/// Health check handler
async fn health_check() -> &'static str {
    "OK"
}

/// Ready check handler
async fn ready_check() -> &'static str {
    "READY"
}


