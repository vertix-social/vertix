use serde::{Serialize, Deserialize};
use aragog::Record;

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
pub struct Account {
    username: String,
    domain: Option<String>,
    uri: Option<String>,
}
