use serde::{Serialize, Deserialize};
use aragog::{Record, DatabaseAccess, DatabaseRecord};
use chrono::{DateTime, Utc};

use crate::{Error, Account, Publish};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recipient {
    Public,
    Account(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
#[before_create(func = "before_create")]
#[before_save(func = "before_save")]
pub struct Note {
    #[serde(default)]
    pub uri: Option<String>,

    #[serde(default)]
    pub to: Vec<Recipient>,

    #[serde(default)]
    pub cc: Vec<Recipient>,

    #[serde(default)]
    pub bto: Vec<Recipient>,

    #[serde(default)]
    pub bcc: Vec<Recipient>,

    pub content: String,

    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
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
            content,
            created_at: None,
            updated_at: None,
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

        DatabaseRecord::link(publisher, &note, db, Publish::default()).await?;

        Ok(note)
    }

    fn before_create(&mut self) -> Result<(), aragog::Error> {
        self.created_at = Some(Utc::now());
        Ok(())
    }

    fn before_save(&mut self) -> Result<(), aragog::Error> {
        self.updated_at = Some(Utc::now());
        Ok(())
    }
}
