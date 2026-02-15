use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Voucher status
#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "varchar")]
pub enum VoucherStatus {
    #[sqlx(rename = "unused")]
    Unused,
    #[sqlx(rename = "redeemed")]
    Redeemed,
    #[sqlx(rename = "expired")]
    Expired,
}

impl std::fmt::Display for VoucherStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoucherStatus::Unused => write!(f, "unused"),
            VoucherStatus::Redeemed => write!(f, "redeemed"),
            VoucherStatus::Expired => write!(f, "expired"),
        }
    }
}

/// Voucher record in database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Voucher {
    pub id: Uuid,
    pub code: String,
    pub usdc_amount: i64, // Amount in cents (6 decimal places = micro USDC)
    pub status: String,
    pub redeemed_by: Option<String>,
    pub redeemed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Voucher {
    /// Get USDC amount as f64
    pub fn usdc_as_f64(&self) -> f64 {
        self.usdc_amount as f64 / 1_000_000.0
    }

    /// Check if voucher is valid for redemption
    pub fn is_valid(&self) -> bool {
        self.status == "unused" && 
            self.expires_at.map_or(true, |exp| exp > Utc::now())
    }
}

/// Voucher repository for database operations
#[derive(Clone)]
pub struct VoucherRepository {
    pool: PgPool,
}

impl VoucherRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find voucher by code
    pub async fn find_by_code(&self, code: &str) -> Result<Option<Voucher>, sqlx::Error> {
        sqlx::query_as::<_, Voucher>(
            "SELECT id, code, usdc_amount, status, redeemed_by, redeemed_at, expires_at, created_at 
             FROM vouchers WHERE UPPER(code) = UPPER($1)"
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
    }

    /// Redeem a voucher for a user
    pub async fn redeem(&self, code: &str, phone: &str) -> Result<Voucher, VoucherError> {
        // First, find and validate the voucher
        let voucher = self.find_by_code(code).await
            .map_err(|e| VoucherError::DatabaseError(e.to_string()))?
            .ok_or(VoucherError::NotFound)?;

        if voucher.status == "redeemed" {
            return Err(VoucherError::AlreadyRedeemed);
        }

        if voucher.status == "expired" || 
           voucher.expires_at.map_or(false, |exp| exp <= Utc::now()) {
            return Err(VoucherError::Expired);
        }

        // Update voucher status
        sqlx::query(
            "UPDATE vouchers SET status = 'redeemed', redeemed_by = $1, redeemed_at = NOW() 
             WHERE id = $2 AND status = 'unused'"
        )
        .bind(phone)
        .bind(voucher.id)
        .execute(&self.pool)
        .await
        .map_err(|e| VoucherError::DatabaseError(e.to_string()))?;

        // Return updated voucher
        self.find_by_code(code).await
            .map_err(|e| VoucherError::DatabaseError(e.to_string()))?
            .ok_or(VoucherError::DatabaseError("Failed to fetch updated voucher".to_string()))
    }

    /// Create a batch of vouchers (admin function)
    pub async fn create_batch(
        &self,
        codes: &[String],
        usdc_amount: i64,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<Vec<Voucher>, sqlx::Error> {
        let mut vouchers = Vec::new();

        for code in codes {
            let id = Uuid::new_v4();
            let voucher = sqlx::query_as::<_, Voucher>(
                r#"
                INSERT INTO vouchers (id, code, usdc_amount, status, expires_at)
                VALUES ($1, $2, $3, 'unused', $4)
                RETURNING id, code, usdc_amount, status, redeemed_by, redeemed_at, expires_at, created_at
                "#
            )
            .bind(id)
            .bind(code.to_uppercase())
            .bind(usdc_amount)
            .bind(expires_at)
            .fetch_one(&self.pool)
            .await?;

            vouchers.push(voucher);
        }

        Ok(vouchers)
    }

    /// Generate random voucher codes
    pub fn generate_codes(count: usize, prefix: &str) -> Vec<String> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        (0..count)
            .map(|_| {
                let random: u32 = rng.gen_range(100000..999999);
                format!("{}{}", prefix, random)
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub enum VoucherError {
    NotFound,
    AlreadyRedeemed,
    Expired,
    DatabaseError(String),
}

impl std::fmt::Display for VoucherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoucherError::NotFound => write!(f, "Voucher not found"),
            VoucherError::AlreadyRedeemed => write!(f, "Voucher already redeemed"),
            VoucherError::Expired => write!(f, "Voucher has expired"),
            VoucherError::DatabaseError(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl std::error::Error for VoucherError {}
