use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::VoucherRepository;

/// Admin routes state
#[derive(Clone)]
pub struct AdminState {
    pub voucher_repo: Arc<VoucherRepository>,
    pub admin_token: String,
}

/// Request to create vouchers
#[derive(Debug, Deserialize)]
pub struct CreateVouchersRequest {
    /// Number of vouchers to create
    pub count: usize,
    /// USDC amount per voucher (e.g., 10.00 for $10)
    pub usdc_amount: f64,
    /// Optional prefix for voucher codes
    #[serde(default = "default_prefix")]
    pub prefix: String,
    /// Optional expiration days from now
    pub expires_in_days: Option<i64>,
}

fn default_prefix() -> String {
    "TTC".to_string()
}

/// Response with created vouchers
#[derive(Debug, Serialize)]
pub struct CreateVouchersResponse {
    pub success: bool,
    pub count: usize,
    pub usdc_amount: f64,
    pub codes: Vec<String>,
}

/// Voucher stats response
#[derive(Debug, Serialize)]
pub struct VoucherStatsResponse {
    pub total: i64,
    pub unused: i64,
    pub redeemed: i64,
    pub total_value_unused: f64,
    pub total_value_redeemed: f64,
}

/// Create admin routes
pub fn admin_routes(state: AdminState) -> Router {
    Router::new()
        .route("/vouchers", post(create_vouchers))
        .route("/vouchers", get(get_voucher_stats))
        .route("/vouchers/list", get(list_vouchers))
        .with_state(state)
}

/// Create new voucher codes
async fn create_vouchers(
    State(state): State<AdminState>,
    Json(req): Json<CreateVouchersRequest>,
) -> Json<CreateVouchersResponse> {
    // Convert USDC to micro USDC (6 decimals)
    let usdc_micro = (req.usdc_amount * 1_000_000.0) as i64;

    // Generate codes
    let codes = VoucherRepository::generate_codes(req.count, &req.prefix);

    // Calculate expiration
    let expires_at = req.expires_in_days.map(|days| {
        chrono::Utc::now() + chrono::Duration::days(days)
    });

    // Create vouchers in database
    match state.voucher_repo.create_batch(&codes, usdc_micro, expires_at).await {
        Ok(vouchers) => {
            let created_codes: Vec<String> = vouchers.iter().map(|v| v.code.clone()).collect();
            Json(CreateVouchersResponse {
                success: true,
                count: created_codes.len(),
                usdc_amount: req.usdc_amount,
                codes: created_codes,
            })
        }
        Err(e) => {
            tracing::error!("Failed to create vouchers: {}", e);
            Json(CreateVouchersResponse {
                success: false,
                count: 0,
                usdc_amount: req.usdc_amount,
                codes: vec![],
            })
        }
    }
}

/// Single voucher info
#[derive(Debug, Serialize)]
pub struct VoucherInfo {
    pub code: String,
    pub usdc_amount: f64,
    pub status: String,
    pub redeemed_by: Option<String>,
}

/// List vouchers response
#[derive(Debug, Serialize)]
pub struct ListVouchersResponse {
    pub vouchers: Vec<VoucherInfo>,
}

/// Get voucher statistics
async fn get_voucher_stats(State(state): State<AdminState>) -> Json<VoucherStatsResponse> {
    // Query stats from database
    let pool = &state.voucher_repo;
    
    // For now, return placeholder - would need to add stats query to repo
    Json(VoucherStatsResponse {
        total: 0,
        unused: 0,
        redeemed: 0,
        total_value_unused: 0.0,
        total_value_redeemed: 0.0,
    })
}

/// List all vouchers (paginated)
async fn list_vouchers(State(_state): State<AdminState>) -> Json<ListVouchersResponse> {
    // Placeholder - would need to add list query to repo
    Json(ListVouchersResponse {
        vouchers: vec![],
    })
}
