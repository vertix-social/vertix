use serde::{Serialize, Deserialize};
use aragog::Record;

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
pub struct Follow {
    accepted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
pub struct Publish {
}

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
pub struct Share {
}

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
pub struct Like {
}
