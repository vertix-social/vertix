use crate::{Error, Note, PageLimit, ApplyPageLimit};
use aragog::{compare, DatabaseAccess, DatabaseRecord, Record, query::{QueryResult, Query, SortDirection}};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use maplit::hashmap;

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
#[before_create(func = "before_create")]
#[before_save(func = "before_save")]
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

    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

impl Account {
    /// Create account data with required fields set.
    pub fn new(username: String) -> Account {
        Account {
            username,
            domain: None,
            uri: None,
            created_at: None,
            updated_at: None,
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
        page_limit: PageLimit,
        db: &D
    ) -> Result<QueryResult<Note>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(Note::get(
            &Query::outbound(1, 1, "Publish", record.id().as_str())
                .apply_page_limit(page_limit)
                .sort("created_at", Some(SortDirection::Desc))
                .with_collections(&["Account", "Publish", "Note"]),
            db
        ).await?)
    }

    /// Get the list of accounts that this account is following.
    pub async fn get_following<D>(
        record: &DatabaseRecord<Account>,
        page_limit: PageLimit,
        db: &D
    ) -> Result<QueryResult<Account>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(Account::get(
            &Query::outbound(1, 1, "Follow", record.id().as_str())
                .apply_page_limit(page_limit)
                .with_collections(&["Account", "Follow"]),
            db
        ).await?)
    }

    /// Get the list of accounts that are following this account.
    pub async fn get_followers<D>(
        record: &DatabaseRecord<Account>,
        page_limit: PageLimit,
        db: &D
    ) -> Result<QueryResult<Account>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(Account::get(
            &Query::inbound(1, 1, "Follow", record.id().as_str())
                .apply_page_limit(page_limit)
                .with_collections(&["Account", "Follow"]),
            db
        ).await?)
    }

    /// Get the latest posts that this account's followers have published or announced.
    pub async fn get_timeline<D>(
        record: &DatabaseRecord<Account>,
        page_limit: PageLimit,
        db: &D
    ) -> Result<QueryResult<Note>, Error> 
    where
        D: DatabaseAccess,
    {
        // Really impossible to do this efficiently without a raw query
        let res: Vec<DatabaseRecord<Note>> = db.database()
            .aql_bind_vars(r#"
                WITH Account, Follow, Publish, Share, Note
                FOR account IN Account
                    FILTER account._key == @account_key
                    FOR note, edge, path IN 2..2 OUTBOUND account Follow, Publish, Share
                        SORT path.edges[1].created_at DESC
                        LIMIT @offset, @limit
                        RETURN note
            "#, hashmap! {
                "account_key" => json!(record.key().as_str()),
                "offset" => json!(page_limit.offset()),
                "limit" => json!(page_limit.limit)
            })
            .await.map_err(aragog::Error::from)?;
        Ok(QueryResult(res))
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
