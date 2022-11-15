use aragog::DatabaseConnection;
use async_trait::async_trait;
use bb8::ManageConnection;
use crate::Error;

pub async fn create_connection() -> Result<DatabaseConnection, Error> {
    let db = DatabaseConnection::builder()
        .build()
        .await?;
    Ok(db)
}

pub struct AragogConnectionManager;

#[async_trait]
impl ManageConnection for AragogConnectionManager {
    type Connection = DatabaseConnection;
    type Error = Error;

    /// Attempts to create a new connection.
    async fn connect(&self) -> Result<DatabaseConnection, Self::Error> {
        create_connection().await
    }

    /// Determines if the connection is still connected to the database.
    async fn is_valid(&self, conn: &mut DatabaseConnection) -> Result<(), Self::Error> {
        let _ = conn.transactions_count().await?;
        Ok(())
    }

    /// Synchronously determine if the connection is no longer usable, if possible.
    fn has_broken(&self, _conn: &mut DatabaseConnection) -> bool {
        // I don't have a way to check
        false
    }
}
