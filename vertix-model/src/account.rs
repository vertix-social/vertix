use crate::{Error, Note, PageLimit, ApplyPageLimit, activitystreams::{ToObject, UrlFor}};
use async_trait::async_trait;
use activitystreams::{ext::Ext, actor::{Person, properties::ApActorProperties}};
use aragog::{compare, DatabaseAccess, DatabaseRecord, Record};
use aragog::query::{QueryResult, Query, SortDirection};
use chrono::{DateTime, Utc, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::json;
use maplit::hashmap;
use url::Url;

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

    /// Information about a remote user. None if local.
    #[serde(default)]
    pub remote: Option<RemoteAccountInfo>,

    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteAccountInfo {
    /// The uri at which the user's data can be found.
    pub uri: Url,

    /// The last time the remote user was fetched.
    #[serde(default)]
    pub last_remote_fetched_at: Option<String>,

    /// Inbox url for remote user.
    #[serde(default)]
    pub inbox: Option<Url>,

    /// Outbox url for remote user.
    #[serde(default)]
    pub outbox: Option<Url>,

    /// Followers url for remote user.
    #[serde(default)]
    pub followers: Option<Url>,

    /// Following url for remote user.
    #[serde(default)]
    pub following: Option<Url>,
    
}

impl Account {
    /// Create account data with required fields set.
    pub fn new(username: String) -> Account {
        Account {
            username,
            domain: None,
            remote: None,
            created_at: None,
            updated_at: None,
        }
    }

    pub fn is_local(&self) -> bool {
        self.domain.is_none()
    }

    pub fn is_remote(&self) -> bool {
        self.domain.is_some()
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

#[async_trait(?Send)]
impl ToObject for DatabaseRecord<Account> {
    type Output = Ext<Person, ApActorProperties>;
    type Error = crate::activitystreams::Error;

    async fn to_object<U, E>(&self, urls: &U) -> Result<Self::Output, E>
    where
        U: UrlFor,
        E: From<Self::Error> + From<U::Error>,
    {
        let mut person = Person::new();
        let mut actor_properties = ApActorProperties::default();

        let account_url = urls.url_for_account(self.key()).await?;
        let inbox_url = urls.url_for_account_inbox(self.key()).await?;
        let outbox_url = urls.url_for_account_outbox(self.key()).await?;

        (|| {
            let o = &mut person.object_props;

            o.set_id(account_url)?;
            o.set_context_xsd_any_uri(activitystreams::context())?;
            o.set_name_xsd_string(self.username.to_owned())?;

            if let Some(created_at) = self.created_at.clone() {
                o.set_published(DateTime::<FixedOffset>::from(created_at))?;
            }

            actor_properties.set_preferred_username(self.username.clone())?;
            actor_properties.set_inbox(inbox_url)?;
            actor_properties.set_outbox(outbox_url)?;

            Ok::<_, Self::Error>(())
        })()?;

        Ok(Ext { base: person, extension: actor_properties })
    }
}

impl RemoteAccountInfo {
    pub fn new(uri: Url) -> RemoteAccountInfo {
        RemoteAccountInfo {
            uri,
            last_remote_fetched_at: None,
            inbox: None,
            outbox: None,
            followers: None,
            following: None,
        }
    }
}
