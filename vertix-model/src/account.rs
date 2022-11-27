use crate::{Error, Note, PageLimit, ApplyPageLimit, activitystreams::{ToObject, UrlFor}, Document, Wrap};
use async_trait::async_trait;
use activitystreams::{
    ext::Ext,
    actor::{Person, properties::ApActorProperties},
    endpoint::EndpointProperties
};
use aragog::{compare, DatabaseAccess, Record};
use aragog::query::{Query, SortDirection};
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
    pub last_fetched_at: Option<DateTime<Utc>>,

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
    ) -> Result<Document<Account>, Error>
    where
        D: DatabaseAccess,
    {
        Account::get(
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
        .wrap()
        .ok_or_else(|| Error::NotFound {
            model: "Account".into(),
            params: json!({"username": username, "domain": domain})
        })
    }

    /// Find an account by their URI. This only works for foreign accounts.
    pub async fn find_by_uri<D>(uri: &Url, db: &D) -> Result<Document<Account>, Error>
    where
        D: DatabaseAccess,
    {
        Account::get(
            &Account::query()
                .bind_var("uri", uri.to_string())
                .filter(compare!(field "remote.uri").equals("@uri").into()),
            db,
        )
        .await?
        .first_record()
        .wrap()
        .ok_or_else(|| Error::NotFound {
            model: "Account".into(),
            params: json!({"uri": uri})
        })
    }

    /// Get the notes that this account has published.
    pub async fn get_published_notes<D>(
        record: &Document<Account>,
        page_limit: PageLimit,
        db: &D
    ) -> Result<Vec<Document<Note>>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(Note::get(
            &Query::outbound(1, 1, "Publish", record.id().as_str())
                .apply_page_limit(page_limit)
                .sort("created_at", Some(SortDirection::Desc))
                .with_collections(&["Account", "Publish", "Note"]),
            db
        ).await?.wrap())
    }

    /// Count the number of notes that this account has published.
    pub async fn count_published_notes<D>(
        record: &Document<Account>,
        db: &D
    ) -> Result<u64, Error>
    where
        D: DatabaseAccess,
    {
        let mut res: Vec<u64> = db.database()
            .aql_bind_vars(r#"
                WITH Publish
                FOR edge in Publish
                    FILTER edge._from == @account_id
                        AND is_same_collection("Note", edge._to)
                    COLLECT WITH COUNT INTO length
                    RETURN length
            "#, hashmap! {
                "account_id" => json!(record.id())
            })
            .await.map_err(aragog::Error::from)?;
        Ok(res.pop().unwrap_or(0))
    }

    /// Get the list of accounts that this account is following.
    pub async fn get_following<D>(
        record: &Document<Account>,
        page_limit: PageLimit,
        db: &D
    ) -> Result<Vec<Document<Account>>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(Account::get(
            &Query::outbound(1, 1, "Follow", record.id().as_str())
                .apply_page_limit(page_limit)
                .with_collections(&["Account", "Follow"]),
            db
        ).await?.wrap())
    }

    /// Count the number of accounts that this account is following.
    pub async fn count_following<D>(
        record: &Document<Account>,
        db: &D
    ) -> Result<u64, Error>
    where
        D: DatabaseAccess,
    {
        let mut res: Vec<u64> = db.database()
            .aql_bind_vars(r#"
                WITH Follow
                FOR edge in Follow
                    FILTER edge._from == @account_id
                    COLLECT WITH COUNT INTO length
                    RETURN length
            "#, hashmap! {
                "account_id" => json!(record.id())
            })
            .await.map_err(aragog::Error::from)?;
        Ok(res.pop().unwrap_or(0))
    }

    /// Get the list of accounts that are following this account.
    pub async fn get_followers<D>(
        record: &Document<Account>,
        page_limit: PageLimit,
        db: &D
    ) -> Result<Vec<Document<Account>>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(Account::get(
            &Query::inbound(1, 1, "Follow", record.id().as_str())
                .apply_page_limit(page_limit)
                .with_collections(&["Account", "Follow"]),
            db
        ).await?.wrap())
    }

    /// Count the number of accounts that are following this account.
    pub async fn count_followers<D>(
        record: &Document<Account>,
        db: &D
    ) -> Result<u64, Error>
    where
        D: DatabaseAccess,
    {
        let mut res: Vec<u64> = db.database()
            .aql_bind_vars(r#"
                WITH Follow
                FOR edge in Follow
                    FILTER edge._to == @account_id
                    COLLECT WITH COUNT INTO length
                    RETURN length
            "#, hashmap! {
                "account_id" => json!(record.id())
            })
            .await.map_err(aragog::Error::from)?;
        Ok(res.pop().unwrap_or(0))
    }

    /// Get the latest posts that this account's followers have published or announced.
    pub async fn get_timeline<D>(
        record: &Document<Account>,
        page_limit: PageLimit,
        db: &D
    ) -> Result<Vec<Document<Note>>, Error> 
    where
        D: DatabaseAccess,
    {
        // Really impossible to do this efficiently without a raw query
        let res: Vec<Document<Note>> = db.database()
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
        Ok(res)
    }

    fn before_create(&mut self) -> Result<(), aragog::Error> {
        if self.is_local() {
            self.created_at = Some(Utc::now());
        }
        Ok(())
    }

    fn before_save(&mut self) -> Result<(), aragog::Error> {
        if self.is_local() {
            self.updated_at = Some(Utc::now());
        }
        Ok(())
    }
}

#[async_trait(?Send)]
impl ToObject for Document<Account> {
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
        let followers_url = urls.url_for_account_followers(self.key()).await?;
        let following_url = urls.url_for_account_following(self.key()).await?;
        let shared_inbox_url = urls.url_for_shared_inbox()?;

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
            actor_properties.set_followers(followers_url)?;
            actor_properties.set_following(following_url)?;
            actor_properties.set_endpoints(EndpointProperties {
                shared_inbox: Some(shared_inbox_url.into()),
                ..Default::default()
            })?;

            Ok::<_, Self::Error>(())
        })()?;

        Ok(Ext { base: person, extension: actor_properties })
    }
}

impl TryFrom<Ext<Person, ApActorProperties>> for Account {
    type Error = crate::error::Error;

    fn try_from(person: Ext<Person, ApActorProperties>) -> Result<Self, Self::Error> {
        let missing = |s: &'static str| Error::ConversionMissingField(s.into());
        let id = person.base.object_props.get_id().ok_or(missing("id"))?;

        Ok(Account {
            username: person.extension.get_preferred_username()
                .ok_or(missing("preferred_username"))?.clone().into_string(),
            // This is just a best guess at the domain. It can be refined by a webfinger.
            domain: id.as_url().host_str().map(|s| s.to_owned()),
            remote: Some(RemoteAccountInfo {
                uri: id.as_url().clone(),
                // we don't know, whoever is consuming this should set it.
                last_fetched_at: None,
                inbox: Some(person.extension.get_inbox().as_url().clone()),
                outbox: Some(person.extension.get_outbox().as_url().clone()),
                followers: person.extension.get_followers().map(|u| u.as_url().clone()),
                following: person.extension.get_following().map(|u| u.as_url().clone()),
            }),
            created_at: person.base.object_props.get_published()
                .map(|d| d.as_datetime().clone().into()),
            updated_at: person.base.object_props.get_updated()
                .map(|d| d.as_datetime().clone().into()),
        })
    }
}

impl RemoteAccountInfo {
    pub fn new(uri: Url) -> RemoteAccountInfo {
        RemoteAccountInfo {
            uri,
            last_fetched_at: None,
            inbox: None,
            outbox: None,
            followers: None,
            following: None,
        }
    }
}
