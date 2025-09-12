use anyhow::{Context, Result};
use tokio_postgres::{Client, NoTls, Statement};

/// Database connection wrapper for PostgreSQL operations
pub struct DatabaseConnection {
    client: Client,
}

impl DatabaseConnection {
    /// Create a new database connection
    pub async fn new(database_url: &str) -> Result<Self> {
        let (client, connection) = tokio_postgres::connect(database_url, NoTls)
            .await
            .with_context(|| format!("Failed to connect to database: {}", database_url))?;

        // Spawn the connection in the background
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Database connection error: {}", e);
            }
        });

        Ok(Self { client })
    }

    /// Prepare a SQL statement and return it
    pub async fn prepare(&mut self, sql: &str) -> Result<Statement> {
        self.client
            .prepare(sql)
            .await
            .with_context(|| format!("Failed to prepare SQL statement: {}", sql))
    }

    /// Get the underlying client for advanced operations
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Test the database connection
    pub async fn test_connection(&mut self) -> Result<()> {
        self.client
            .execute("SELECT 1", &[])
            .await
            .with_context(|| "Failed to test database connection")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a PostgreSQL database to be running
    // You can skip them in CI/CD by using `cargo test --lib`
    
    #[tokio::test]
    #[ignore] // Ignore by default since it requires a database
    async fn test_database_connection() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/test".to_string());
        
        let mut db = DatabaseConnection::new(&database_url).await.unwrap();
        db.test_connection().await.unwrap();
    }
}
