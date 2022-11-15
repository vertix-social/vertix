use aragog::DatabaseConnection;

mod account;
mod note;
mod edges;
mod error;

pub use account::*;
pub use note::*;
pub use edges::*;
pub use error::*;

pub async fn create_connection() -> Result<DatabaseConnection, Error> {
    let db = DatabaseConnection::builder()
        .build()
        .await?;

    Ok(db)
}
