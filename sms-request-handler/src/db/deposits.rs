use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Deposit source type
#[derive(Debug, Clone, PartialEq)]
pub enum DepositSource {
    Voucher,
    OnChain,
    Partner,
}

impl std::fmt::Display for DepositSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DepositSource::Voucher => write!(f, "voucher"),
            DepositSource::OnChain => write!(f, "onchain"),
            DepositSource::Partner => write!(f, "partner"),
        }
    }
}

/// Deposit record in database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Deposit {
    pub id: Uuid,
    pub user_phone: String,
    pub amount: i64,          // Amount in micro USDC (6 decimals)
    pub source: String,       // "voucher", "onchain", "partner"
    pub source_ref: Option<String>,  // voucher code, tx hash, or partner ref
    pub chain: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Deposit {
    /// Get amount as f64 (human readable)
    pub fn amount_as_f64(&self) -> f64 {
        self.amount as f64 / 1_000_000.0
    }
}

/// Deposit repository for database operations
#[derive(Clone)]
pub struct DepositRepository {
    pool: PgPool,
}

impl DepositRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Record a new deposit from voucher redemption
    pub async fn create_from_voucher(
        &self,
        phone: &str,
        amount: i64,
        voucher_code: &str,
    ) -> Result<Deposit, sqlx::Error> {
        let id = Uuid::new_v4();
        
        sqlx::query_as::<_, Deposit>(
            r#"
            INSERT INTO deposits (id, user_phone, amount, source, source_ref)
            VALUES ($1, $2, $3, 'voucher', $4)
            RETURNING id, user_phone, amount, source, source_ref, chain, created_at
            "#
        )
        .bind(id)
        .bind(phone)
        .bind(amount)
        .bind(voucher_code)
        .fetch_one(&self.pool)
        .await
    }

    /// Record an on-chain deposit
    pub async fn create_from_chain(
        &self,
        phone: &str,
        amount: i64,
        tx_hash: &str,
        chain: &str,
    ) -> Result<Deposit, sqlx::Error> {
        let id = Uuid::new_v4();
        
        sqlx::query_as::<_, Deposit>(
            r#"
            INSERT INTO deposits (id, user_phone, amount, source, source_ref, chain)
            VALUES ($1, $2, $3, 'onchain', $4, $5)
            RETURNING id, user_phone, amount, source, source_ref, chain, created_at
            "#
        )
        .bind(id)
        .bind(phone)
        .bind(amount)
        .bind(tx_hash)
        .bind(chain)
        .fetch_one(&self.pool)
        .await
    }

    /// Get all deposits for a user
    pub async fn find_by_user(&self, phone: &str) -> Result<Vec<Deposit>, sqlx::Error> {
        sqlx::query_as::<_, Deposit>(
            "SELECT id, user_phone, amount, source, source_ref, chain, created_at 
             FROM deposits WHERE user_phone = $1 ORDER BY created_at DESC"
        )
        .bind(phone)
        .fetch_all(&self.pool)
        .await
    }

    /// Get total USDC balance for a user (from all deposits)
    pub async fn get_balance(&self, phone: &str) -> Result<i64, sqlx::Error> {
        let result = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(SUM(amount), 0) FROM deposits WHERE user_phone = $1"
        )
        .bind(phone)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(result)
    }

    /// Get balance as formatted string
    pub async fn get_balance_formatted(&self, phone: &str) -> Result<String, sqlx::Error> {
        let balance = self.get_balance(phone).await?;
        let usdc = balance as f64 / 1_000_000.0;
        Ok(format!("{:.2}", usdc))
    }

    /// Get recent deposits (last N)
    pub async fn get_recent(&self, phone: &str, limit: i64) -> Result<Vec<Deposit>, sqlx::Error> {
        sqlx::query_as::<_, Deposit>(
            "SELECT id, user_phone, amount, source, source_ref, chain, created_at 
             FROM deposits WHERE user_phone = $1 
             ORDER BY created_at DESC LIMIT $2"
        )
        .bind(phone)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }
}
