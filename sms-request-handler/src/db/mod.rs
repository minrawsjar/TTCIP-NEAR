pub mod address_book;
pub mod deposits;
pub mod users;
pub mod vouchers;

pub use address_book::*;
pub use deposits::*;
pub use users::*;
pub use vouchers::*;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Create a database connection pool
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    tracing::info!("Creating users table...");
    // Users table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY,
            phone VARCHAR(20) UNIQUE NOT NULL,
            wallet_address VARCHAR(42) NOT NULL,
            encrypted_private_key TEXT NOT NULL,
            pin_hash VARCHAR(255),
            ens_name VARCHAR(255),
            preferred_chain VARCHAR(20) DEFAULT 'polygon-amoy',
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    tracing::info!("Creating indices for users...");
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_phone ON users(phone)")
        .execute(pool)
        .await?;
    
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_wallet ON users(wallet_address)")
        .execute(pool)
        .await?;

    tracing::info!("Creating vouchers table...");
    // Vouchers table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS vouchers (
            id UUID PRIMARY KEY,
            code VARCHAR(20) UNIQUE NOT NULL,
            usdc_amount BIGINT NOT NULL,
            status VARCHAR(20) NOT NULL DEFAULT 'unused',
            redeemed_by VARCHAR(20),
            redeemed_at TIMESTAMP WITH TIME ZONE,
            expires_at TIMESTAMP WITH TIME ZONE,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    tracing::info!("Creating indices for vouchers...");
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_vouchers_code ON vouchers(code)")
        .execute(pool)
        .await?;
    
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_vouchers_status ON vouchers(status)")
        .execute(pool)
        .await?;

    tracing::info!("Creating deposits table...");
    // Deposits table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS deposits (
            id UUID PRIMARY KEY,
            user_phone VARCHAR(20) NOT NULL,
            amount BIGINT NOT NULL,
            source VARCHAR(20) NOT NULL,
            source_ref VARCHAR(255),
            chain VARCHAR(30),
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    tracing::info!("Creating indices for deposits...");
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_deposits_user ON deposits(user_phone)")
        .execute(pool)
        .await?;
    
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_deposits_source ON deposits(source)")
        .execute(pool)
        .await?;

    tracing::info!("Creating address_book table...");
    // Address book table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS address_book (
            id UUID PRIMARY KEY,
            user_phone VARCHAR(20) NOT NULL,
            name VARCHAR(50) NOT NULL,
            contact_phone VARCHAR(50),
            wallet_address VARCHAR(42),
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    tracing::info!("Creating indices for address_book...");
    // Ensure unique constraint exists (using index for flexibility with nulls)
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_address_book_entries 
         ON address_book (user_phone, COALESCE(contact_phone, ''), COALESCE(wallet_address, ''))"
    )
    .execute(pool)
    .await?;

    // Fix column size if it was created with VARCHAR(20)
    // We ignore error if it fails (e.g. DB doesn't support generic ALTER or already done)
    let _ = sqlx::query("ALTER TABLE address_book ALTER COLUMN contact_phone TYPE VARCHAR(50)")
        .execute(pool)
        .await;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_address_book_user ON address_book(user_phone)")
        .execute(pool)
        .await?;
    
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_address_book_name ON address_book(user_phone, name)")
        .execute(pool)
        .await?;

    tracing::info!("Database migrations completed");
    Ok(())
}

