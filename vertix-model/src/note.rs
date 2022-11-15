use serde::{Serialize, Deserialize};
use aragog::{Record, DatabaseAccess, DatabaseRecord};

use crate::{Error, Account, Publish};

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
pub struct Note {
    #[serde(default)]
    pub uri: Option<String>,

    #[serde(default)]
    pub to: Vec<String>,

    #[serde(default)]
    pub cc: Vec<String>,

    #[serde(default)]
    pub bto: Vec<String>,

    #[serde(default)]
    pub bcc: Vec<String>,

    pub content: String,
}

impl Note {
    /// Create note data with required fields set.
    pub fn new(content: String) -> Note {
        Note {
            uri: None,
            to: vec![],
            cc: vec![],
            bto: vec![],
            bcc: vec![],
            content
        }
    }

    /// Publish a new note from the publisher. Creates a Note record and Publish edge.
    pub async fn publish<D>(
        publisher: &DatabaseRecord<Account>,
        note: Note,
        db: &D
    ) -> Result<DatabaseRecord<Note>, Error>
    where
        D: DatabaseAccess,
    {
        let note = Note::create(note, db).await?;

        DatabaseRecord::link(publisher, &note, db, Publish { }).await?;

        Ok(note)
    }
}
