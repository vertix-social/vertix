use serde::{Serialize, Deserialize};
use aragog::{Record, EdgeRecord, DatabaseRecord, DatabaseAccess, compare};
use chrono::{DateTime, Utc};
use crate::{Account, Error};

macro_rules! created_at_hook {
    () => {
        fn before_create(&mut self) -> Result<(), aragog::Error> {
            self.created_at = Some(Utc::now());
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Record)]
#[before_create(func = "before_create")]
#[serde(default)]
pub struct Follow {
    pub accepted: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
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

    created_at_hook!();
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Record)]
#[before_create(func = "before_create")]
#[serde(default)]
pub struct Publish {
    pub created_at: Option<DateTime<Utc>>,
}

impl Publish {
    created_at_hook!();
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Record)]
#[before_create(func = "before_create")]
#[serde(default)]
pub struct Share {
    pub created_at: Option<DateTime<Utc>>,
}

impl Share {
    created_at_hook!();
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Record)]
#[before_create(func = "before_create")]
#[serde(default)]
pub struct Like {
    pub created_at: Option<DateTime<Utc>>,
}

impl Like {
    created_at_hook!();
}
