use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Contact in address book
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Contact {
    pub id: Uuid,
    pub user_phone: String,      // Owner of this contact
    pub name: String,            // Contact name/label
    pub contact_phone: Option<String>,  // Phone number if known
    pub wallet_address: Option<String>, // Wallet address if known
    pub created_at: DateTime<Utc>,
}

impl Contact {
    /// Format for SMS display
    pub fn to_sms_string(&self) -> String {
        match (&self.contact_phone, &self.wallet_address) {
            (Some(phone), _) => format!("{}: {}", self.name, phone),
            (_, Some(addr)) => format!("{}: {}...{}", self.name, &addr[..6], &addr[38..]),
            _ => self.name.clone(),
        }
    }
}

/// Address book repository for database operations
#[derive(Clone)]
pub struct AddressBookRepository {
    pool: PgPool,
}

impl AddressBookRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Add a new contact
    pub async fn add_contact(
        &self,
        user_phone: &str,
        name: &str,
        contact_phone: Option<&str>,
        wallet_address: Option<&str>,
    ) -> Result<Contact, sqlx::Error> {
        let id = Uuid::new_v4();
        
        sqlx::query_as::<_, Contact>(
            r#"
            INSERT INTO address_book (id, user_phone, name, contact_phone, wallet_address)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_phone, COALESCE(contact_phone, ''), COALESCE(wallet_address, ''))
            DO UPDATE SET name = EXCLUDED.name
            RETURNING id, user_phone, name, contact_phone, wallet_address, created_at
            "#
        )
        .bind(id)
        .bind(user_phone)
        .bind(name)
        .bind(contact_phone)
        .bind(wallet_address)
        .fetch_one(&self.pool)
        .await
    }

    /// Find contacts by name (partial match)
    pub async fn find_by_name(&self, user_phone: &str, name: &str) -> Result<Vec<Contact>, sqlx::Error> {
        sqlx::query_as::<_, Contact>(
            "SELECT id, user_phone, name, contact_phone, wallet_address, created_at 
             FROM address_book 
             WHERE user_phone = $1 AND UPPER(name) LIKE UPPER($2)
             ORDER BY name"
        )
        .bind(user_phone)
        .bind(format!("%{}%", name))
        .fetch_all(&self.pool)
        .await
    }

    /// Find contact by phone number
    pub async fn find_by_phone(&self, user_phone: &str, contact_phone: &str) -> Result<Option<Contact>, sqlx::Error> {
        sqlx::query_as::<_, Contact>(
            "SELECT id, user_phone, name, contact_phone, wallet_address, created_at 
             FROM address_book 
             WHERE user_phone = $1 AND contact_phone = $2"
        )
        .bind(user_phone)
        .bind(contact_phone)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get all contacts for a user
    pub async fn list_all(&self, user_phone: &str) -> Result<Vec<Contact>, sqlx::Error> {
        sqlx::query_as::<_, Contact>(
            "SELECT id, user_phone, name, contact_phone, wallet_address, created_at 
             FROM address_book 
             WHERE user_phone = $1 
             ORDER BY name"
        )
        .bind(user_phone)
        .fetch_all(&self.pool)
        .await
    }

    /// Delete a contact
    pub async fn delete(&self, user_phone: &str, name: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM address_book WHERE user_phone = $1 AND UPPER(name) = UPPER($2)"
        )
        .bind(user_phone)
        .bind(name)
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected() > 0)
    }

    /// Resolve a recipient - could be a name, phone, or address
    pub async fn resolve_recipient(&self, user_phone: &str, input: &str) -> Option<String> {
        // If it looks like a phone number or address, return as-is
        if input.starts_with('+') || input.starts_with("0x") {
            return Some(input.to_string());
        }

        // Try to find in address book by name
        let contacts = self.find_by_name(user_phone, input).await.ok()?;
        
        contacts.first().and_then(|c| {
            c.contact_phone.clone().or(c.wallet_address.clone())
        })
    }
}
