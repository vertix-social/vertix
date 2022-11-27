use activitystreams::activity;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use aragog::{Record, EdgeRecord, DatabaseRecord, DatabaseAccess, compare};
use chrono::{DateTime, Utc};
use serde_json::json;
use url::Url;
use crate::{Account, Error, Edge, Wrap, activitystreams::{ToObject, UrlFor}};

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
    /// `None` if pending, `Some(true)` if accepted, `Some(false)` if rejected.
    pub accepted: Option<bool>,

    /// True if the follow came from a remote actor.
    pub from_remote: bool,

    /// True if the follow is to a remote object.
    pub to_remote: bool,

    /// Set if the remote provided a uri.
    pub uri: Option<Url>,

    /// When the follow was created.
    pub created_at: Option<DateTime<Utc>>,
}

impl Follow {
    /// Initiate follow request between two accounts.
    pub async fn link<D>(
        actor: &DatabaseRecord<Account>,
        target: &DatabaseRecord<Account>,
        uri: Option<Url>,
        db: &D
    ) -> Result<Edge<Follow>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(DatabaseRecord::link(
            actor,
            target,
            db,
            Follow {
                from_remote: actor.is_remote(),
                to_remote: target.is_remote(),
                uri,
                ..Follow::default()
            }
        ).await?.wrap())
    }

    /// Find follow between two accounts.
    pub async fn find_between<D>(
        actor: &DatabaseRecord<Account>,
        target: &DatabaseRecord<Account>,
        db: &D
    ) -> Result<Edge<Follow>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(EdgeRecord::get(
            &Follow::query()
                .bind_var("from", actor.id().as_str())
                .bind_var("to", target.id().as_str())
                .filter(
                    compare!(field "_from").equals("@from")
                        .and(compare!(field "_to").equals("@to")).into()),
            db
        ).await?.first_record().wrap().ok_or_else(|| Error::NotFound {
            model: "Follow".into(),
            params: json!({"_from": actor.id(), "_to": target.id()})
        })?)
    }

    // Find pending follows from an account.
    pub async fn find_pending_from<D>(from: &DatabaseRecord<Account>, db: &D)
        -> Result<Vec<Edge<Follow>>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(EdgeRecord::get(
            &Follow::query()
                .bind_var("from", from.id().as_str())
                .filter(compare!(field "_from").equals("@from")
                    .and(compare!(field "accepted").equals("null")).into()),
            db
        ).await?.wrap())
    }

    // Find pending follows to an account.
    pub async fn find_pending_to<D>(to: &DatabaseRecord<Account>, db: &D)
        -> Result<Vec<Edge<Follow>>, Error>
    where
        D: DatabaseAccess,
    {
        Ok(EdgeRecord::get(
            &Follow::query()
                .bind_var("to", to.id().as_str())
                .filter(compare!(field "_to").equals("@to")
                    .and(compare!(field "accepted").equals("null")).into()),
            db
        ).await?.wrap())
    }

    created_at_hook!();
}

#[async_trait(?Send)]
impl ToObject for Edge<Follow> {
    type Output = activity::Follow;
    type Error = crate::activitystreams::Error;

    async fn to_object<U, E>(&self, urls: &U) -> Result<Self::Output, E>
    where
        U: UrlFor,
        E: From<Self::Error> + From<U::Error>
    {
        let (from_url, to_url) = futures::try_join!(
            urls.url_for_account(self.key_from()),
            urls.url_for_account(self.key_to()),
        )?;

        let mut follow_activity = activity::Follow::new();

        (|| {
            follow_activity.object_props.set_context_xsd_any_uri(activitystreams::context())?;
            follow_activity.object_props.set_id(self.uri.clone().unwrap_or_else(|| {
                let mut url = from_url.clone();
                url.set_fragment(Some(&format!("self/{}", self.key())));
                url
            }))?;
            follow_activity.follow_props.set_actor_xsd_any_uri(from_url)?;
            follow_activity.follow_props.set_object_xsd_any_uri(to_url)?;
            Ok::<_, crate::activitystreams::Error>(())
        })()?;

        Ok(follow_activity)
    }
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
