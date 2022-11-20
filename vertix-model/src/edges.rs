use serde::{Serialize, Deserialize};
use aragog::{Record, EdgeRecord, DatabaseRecord, DatabaseAccess, compare};
use chrono::{DateTime, Utc};
use crate::{Account, Error};

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
pub struct Follow {
    #[serde(default)]
    pub accepted: Option<bool>,
    pub created_at: DateTime<Utc>,
}

impl Follow {
    pub fn new() -> Follow {
        Follow { accepted: None, created_at: Utc::now() }
    }

    /// Initiate follow request between two accounts.
    pub async fn link<D>(
        actor: &DatabaseRecord<Account>,
        target: &DatabaseRecord<Account>,
        db: &D
    ) -> Result<DatabaseRecord<EdgeRecord<Follow>>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(DatabaseRecord::link(
            actor,
            target,
            db,
            Follow::new()
        ).await?)
    }

    /// Find follow between two accounts.
    pub async fn find_between<D>(
        actor: &DatabaseRecord<Account>,
        target: &DatabaseRecord<Account>,
        db: &D
    ) -> Result<DatabaseRecord<EdgeRecord<Follow>>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(EdgeRecord::get(
            &Follow::query()
                .bind_var("from", actor.key().as_str())
                .bind_var("to", target.key().as_str())
                .filter(
                    compare!(field "_from").equals("@from")
                        .and(compare!(field "_to").equals("@to")).into()),
            db
        ).await?.first_record().unwrap())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
pub struct Publish {
    pub created_at: DateTime<Utc>,
}

impl Publish {
    pub fn new() -> Publish {
        Publish { created_at: Utc::now() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
pub struct Share {
    pub created_at: DateTime<Utc>,
}

impl Share {
    pub fn new() -> Share {
        Share { created_at: Utc::now() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
pub struct Like {
    pub created_at: DateTime<Utc>,
}

impl Like {
    pub fn new() -> Like {
        Like { created_at: Utc::now() }
    }
}
