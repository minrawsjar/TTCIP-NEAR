use sqlx::PgPool;
use uuid::Uuid;

/// User record in database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub phone: String,
    pub wallet_address: String,
    pub encrypted_private_key: String,
    pub pin_hash: Option<String>,
    pub ens_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// User repository for database operations
#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find user by phone number
    pub async fn find_by_phone(&self, phone: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, phone, wallet_address, encrypted_private_key, pin_hash, ens_name, created_at 
             FROM users WHERE phone = $1"
        )
        .bind(phone)
        .fetch_optional(&self.pool)
        .await
    }

    /// Create a new user
    pub async fn create(
        &self,
        phone: &str,
        wallet_address: &str,
        encrypted_private_key: &str,
    ) -> Result<User, sqlx::Error> {
        let id = Uuid::new_v4();
        
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, phone, wallet_address, encrypted_private_key)
            VALUES ($1, $2, $3, $4)
            RETURNING id, phone, wallet_address, encrypted_private_key, pin_hash, ens_name, created_at
            "#
        )
        .bind(id)
        .bind(phone)
        .bind(wallet_address)
        .bind(encrypted_private_key)
        .fetch_one(&self.pool)
        .await
    }

    /// Update user's PIN hash
    pub async fn update_pin(&self, phone: &str, pin_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET pin_hash = $1 WHERE phone = $2")
            .bind(pin_hash)
            .bind(phone)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Update user's ENS name
    pub async fn update_ens_name(&self, phone: &str, ens_name: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET ens_name = $1 WHERE phone = $2")
            .bind(ens_name)
            .bind(phone)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Check if user exists
    pub async fn exists(&self, phone: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE phone = $1"
        )
        .bind(phone)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(result > 0)
    }
}
