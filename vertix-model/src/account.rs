use crate::{Error, Note, Publish, Follow};
use aragog::{compare, DatabaseAccess, DatabaseRecord, Record, query::{QueryResult, Query}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
pub struct Account {
    /// The username part of the handle, i.e. the username in a handle `@{username}` or
    /// `@{username}@{domain}`
    pub username: String,

    /// The domain at which the user resides. None if local.
    #[serde(default)]
    pub domain: Option<String>,

    /// The uri at which the user's data can be found. None if local.
    #[serde(default)]
    pub uri: Option<String>,
}

impl Account {
    /// Create account data with required fields set.
    pub fn new(username: String) -> Account {
        Account {
            username,
            domain: None,
            uri: None,
        }
    }

    /// Find an account by username and domain. Use domain = `None` for a local account.
    pub async fn find_by_username<D>(
        username: &str,
        domain: Option<&str>,
        db: &D,
    ) -> Result<DatabaseRecord<Account>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(Account::get(
            &Account::query()
                .bind_var("username", username)
                .bind_var("domain", domain)
                .filter(
                    compare!(field "username").equals("@username")
                        .and(compare!(field "domain").equals("@domain"))
                        .into(),
                ),
            db,
        )
        .await?
        .first_record()
        .unwrap())
    }

    /// Find an account by their URI. This only works for foreign accounts.
    pub async fn find_by_uri<D>(uri: &str, db: &D) -> Result<DatabaseRecord<Account>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(Account::get(
            &Account::query()
                .bind_var("uri", uri)
                .filter(compare!(field "uri").equals("@uri").into()),
            db,
        )
        .await?
        .first_record()
        .unwrap())
    }

    /// Get the notes that this account has published.
    pub async fn get_published_notes<D>(
        record: &DatabaseRecord<Account>,
        db: &D
    ) -> Result<QueryResult<Note>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(Note::get(
            &Account::query()
                .bind_var("account_key", record.key().as_str())
                .filter(compare!(field "_key").equals("@account_key").into())
                .join_outbound(1, 1, false, Publish::query())
                .with_collections(&["Account", "Publish", "Note"]),
            db
        ).await?)
    }

    /// Get the list of accounts that this account is following.
    pub async fn get_following<D>(
        record: &DatabaseRecord<Account>,
        db: &D
    ) -> Result<QueryResult<Account>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(Account::get(
            &Account::query()
                .bind_var("account_key", record.key().as_str())
                .filter(compare!(field "_key").equals("@account_key").into())
                .join_outbound(1, 1, false, Follow::query())
                .with_collections(&["Account", "Follow"]),
            db
        ).await?)
    }

    /// Get the list of accounts that are following this account.
    pub async fn get_followers<D>(
        record: &DatabaseRecord<Account>,
        db: &D
    ) -> Result<QueryResult<Account>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(Account::get(
            &Account::query()
                .bind_var("account_key", record.key().as_str())
                .filter(compare!(field "_key").equals("@account_key").into())
                .join_inbound(1, 1, false, Follow::query())
                .with_collections(&["Account", "Follow"]),
            db
        ).await?)
    }

    /// Get the latest posts that this account's followers have published or announced.
    pub async fn get_timeline<D>(
        record: &DatabaseRecord<Account>,
        db: &D
    ) -> Result<QueryResult<Note>, Error> 
    where
        D: DatabaseAccess,
    {
        Ok(Note::get(
            &Account::query()
                .bind_var("account_key", record.key().as_str())
                .filter(compare!(field "_key").equals("@account_key").into())
                .join_outbound(2, 2, false, Query::new("Follow, Publish, Share"))
                .with_collections(&["Account", "Follow", "Publish", "Share", "Note"]),
            db
        ).await?)
    }
}
