use aragog::DatabaseAccess;
use vertix_model::{RecordCache, Account, Note};
use vertix_model::activitystreams::UrlFor;
use crate::error::*;
use url::Url;
use urlencoding::encode;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct Urls<'a, D> {
    pub base_url: &'a Url,
    pub account_cache: RecordCache<Account>,
    pub note_cache: RecordCache<Note>,
    pub db: &'a D,
}

impl<'a, D> Urls<'a, D> where D: DatabaseAccess {
    pub fn new(base_url: &'a Url, db: &'a D) -> Urls<'a, D> {
        Urls {
            base_url,
            account_cache: RecordCache::new(),
            note_cache: RecordCache::new(),
            db
        }
    }
}

impl<'a, D> Urls<'a, D> where D: DatabaseAccess {
    pub fn is_own_url(&self, url: &Url) -> bool {
        self.base_url.make_relative(url).is_some()
    }
}

macro_rules! url_for_account_suffix_impl {
    ($name:ident ( $self:expr, $key:expr ) = format($fmt:literal)) => ({
        let mut url = $self.url_for_account($key).await?;

        if $self.is_own_url(&url) {
            url.set_path(&format!($fmt, url.path()));
            Ok(url)
        } else {
            // Not owned by us
            Err(Error::InternalError(
                concat!("can't ", stringify!($name), " for remote account").into()))
        }
    })
}

#[async_trait]
impl<'a, D> UrlFor for Urls<'a, D>
where
    D: DatabaseAccess,
{
    type Error = Error;

    async fn url_for_account(&self, key: &str) -> Result<Url> {
        let account = self.account_cache.get(key, self.db).await?;

        if let Some(ref remote) = account.remote {
            Ok(remote.uri.clone())
        } else {
            let username = encode(&account.username);
            Ok(self.base_url.join(&format!("users/{username}"))?)
        }
    }

    async fn url_for_account_inbox(&self, key: &str) -> Result<Url> {
        url_for_account_suffix_impl!(url_for_account_inbox(self, key) =
            format("{}/inbox"))
    }

    async fn url_for_account_outbox(&self, key: &str) -> Result<Url> {
        url_for_account_suffix_impl!(url_for_account_outbox(self, key) =
            format("{}/outbox"))
    }

    async fn url_for_account_outbox_page(&self, key: &str, page: u32) -> Result<Url> {
        url_for_account_suffix_impl!(url_for_account_outbox_page(self, key) =
            format("{}/outbox/page/{page}"))
    }

    async fn url_for_note(&self, key: &str) -> Result<Url> {
        let note = self.note_cache.get(key, self.db).await?;

        if let Some(ref remote) = note.remote {
            Ok(remote.uri.clone())
        } else {
            let from = note.from.as_ref()
                .ok_or(Error::InternalError("note.from is missing".into()))?;
            let mut url = self.url_for_account(&from).await?;
            let encoded_key = encode(key);
            url.set_path(&format!("{}/notes/{encoded_key}", url.path()));
            Ok(url)
        }
    }
}
