use serde::{Serialize, Deserialize};
use aragog::Record;

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
pub struct Note {
    uri: Option<String>,
    to: Vec<String>,
    cc: Vec<String>,
    bto: Vec<String>,
    bcc: Vec<String>,
    content: String,
}
