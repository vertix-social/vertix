use aragog::DatabaseConnection;
use anyhow::Result;
use serde_yaml;

mod account;
mod note;
mod edges;

pub use account::*;
pub use note::*;
pub use edges::*;

static SCHEMA_YAML: &'static str = include_str!("schema.yaml");

pub async fn create_connection() -> Result<DatabaseConnection> {
    let db_schema = serde_yaml::from_str(SCHEMA_YAML).unwrap();

    let db = DatabaseConnection::builder()
        .with_schema(db_schema)
        .apply_schema()
        .build()
        .await?;

    Ok(db)
}
