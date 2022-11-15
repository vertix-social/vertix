use serde::{Serialize, Deserialize};
use aragog::{Record, EdgeRecord, DatabaseRecord, DatabaseAccess, compare};
use crate::{Account, Error};

#[derive(Debug, Clone, Serialize, Deserialize, Record, Default)]
#[serde(default)]
pub struct Follow {
    pub accepted: Option<bool>,
}

impl Follow {
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
            Follow::default()
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

#[derive(Debug, Clone, Serialize, Deserialize, Record, Default)]
#[serde(default)]
pub struct Publish {
}

#[derive(Debug, Clone, Serialize, Deserialize, Record, Default)]
#[serde(default)]
pub struct Share {
}

#[derive(Debug, Clone, Serialize, Deserialize, Record, Default)]
#[serde(default)]
pub struct Like {
}
