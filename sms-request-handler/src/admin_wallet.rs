use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use sqlx::PgPool;
use std::sync::Arc;

/// Wallet info response
#[derive(Debug, Serialize)]
pub struct WalletInfo {
    pub phone: String,
    pub wallet_address: String,
    pub ens_name: Option<String>,
    pub created_at: String,
}

/// List all wallets response
#[derive(Debug, Serialize)]
pub struct ListWalletsResponse {
    pub success: bool,
    pub count: usize,
    pub wallets: Vec<WalletInfo>,
}

/// Get wallet by phone response
#[derive(Debug, Serialize)]
pub struct GetWalletResponse {
    pub success: bool,
    pub wallet: Option<WalletInfo>,
}

/// Admin wallet routes state
#[derive(Clone)]
pub struct AdminWalletState {
    pub db_pool: Arc<PgPool>,
}

/// Create admin wallet routes
pub fn admin_wallet_routes(db_pool: Arc<PgPool>) -> Router {
    let state = AdminWalletState { db_pool };
    
    Router::new()
        .route("/wallets", get(list_all_wallets))
        .route("/wallets/:phone", get(get_wallet_by_phone))
        .with_state(state)
}

/// List all wallets with full addresses
async fn list_all_wallets(
    State(state): State<AdminWalletState>,
) -> Json<ListWalletsResponse> {
    let result = sqlx::query_as::<_, (String, String, Option<String>, chrono::DateTime<chrono::Utc>)>(
        "SELECT phone, wallet_address, ens_name, created_at FROM users ORDER BY created_at DESC LIMIT 100"
    )
    .fetch_all(&*state.db_pool)
    .await;

    match result {
        Ok(rows) => {
            let wallets: Vec<WalletInfo> = rows
                .into_iter()
                .map(|(phone, wallet_address, ens_name, created_at)| WalletInfo {
                    phone,
                    wallet_address,
                    ens_name,
                    created_at: created_at.to_rfc3339(),
                })
                .collect();

            Json(ListWalletsResponse {
                success: true,
                count: wallets.len(),
                wallets,
            })
        }
        Err(e) => {
            tracing::error!("Failed to fetch wallets: {}", e);
            Json(ListWalletsResponse {
                success: false,
                count: 0,
                wallets: vec![],
            })
        }
    }
}

/// Get wallet by phone number
async fn get_wallet_by_phone(
    State(state): State<AdminWalletState>,
    Path(phone): Path<String>,
) -> Json<GetWalletResponse> {
    let result = sqlx::query_as::<_, (String, String, Option<String>, chrono::DateTime<chrono::Utc>)>(
        "SELECT phone, wallet_address, ens_name, created_at FROM users WHERE phone = $1"
    )
    .bind(&phone)
    .fetch_optional(&*state.db_pool)
    .await;

    match result {
        Ok(Some((phone, wallet_address, ens_name, created_at))) => {
            Json(GetWalletResponse {
                success: true,
                wallet: Some(WalletInfo {
                    phone,
                    wallet_address,
                    ens_name,
                    created_at: created_at.to_rfc3339(),
                }),
            })
        }
        Ok(None) => Json(GetWalletResponse {
            success: false,
            wallet: None,
        }),
        Err(e) => {
            tracing::error!("Failed to fetch wallet: {}", e);
            Json(GetWalletResponse {
                success: false,
                wallet: None,
            })
        }
    }
}
