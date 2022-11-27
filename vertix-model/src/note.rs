use activitystreams::object;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use aragog::{Record, DatabaseAccess, DatabaseRecord, Validate};
use chrono::{DateTime, Utc, FixedOffset};
use url::Url;
use futures::stream::{FuturesOrdered, TryStreamExt};

use crate::{Error, Account, Publish, activitystreams::{ToObject, UrlFor}};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recipient {
    Public,
    Account(String),
}

impl Recipient {
    pub async fn url_for<U>(&self, urls: &U) -> Result<Url, U::Error>
    where
        U: UrlFor,
    {
        match self {
            Recipient::Public => Ok(activitystreams::public().into()),
            Recipient::Account(key) => urls.url_for_account(key).await,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Record)]
#[before_create(func = "before_create")]
#[before_save(func = "before_save")]
pub struct Note {
    /// Account key
    #[serde(default)]
    pub from: Option<String>,

    /// Some only if the Note comes from a remote source
    #[serde(default)]
    pub remote: Option<RemoteNoteInfo>,

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteNoteInfo {
    /// The uri of the remote note.
    pub uri: Url,
}

impl Note {
    /// Create note data with required fields set.
    pub fn new(content: String) -> Note {
        Note {
            from: None,
            remote: None,
            to: vec![],
            cc: vec![],
            bto: vec![],
            bcc: vec![],
            content,
            created_at: None,
            updated_at: None,
        }
    }

    pub fn is_local(&self) -> bool {
        self.remote.is_none()
    }

    pub fn is_remote(&self) -> bool {
        self.remote.is_some()
    }

    /// Publish a new note from the publisher. Creates a Note record and Publish edge.
    ///
    /// `note.from` will be set to `publisher.key()`.
    pub async fn publish<D>(
        publisher: &DatabaseRecord<Account>,
        note: Note,
        db: &D
    ) -> Result<DatabaseRecord<Note>, Error>
    where
        D: DatabaseAccess,
    {
        let note = Note::create(Note {
            from: Some(publisher.key().into()),
            ..note
        }, db).await?;

        DatabaseRecord::link(publisher, &note, db, Publish::default()).await?;

        Ok(note)
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

impl Validate for Note {
    fn validations(&self, errors: &mut Vec<String>) {
        if self.from.is_none() {
            errors.push("note.from must be set".into());
        }
    }
}

#[async_trait(?Send)]
impl ToObject for DatabaseRecord<Note> {
    type Output = object::Note;
    type Error = crate::activitystreams::Error;

    async fn to_object<U, E>(&self, urls: &U) -> Result<Self::Output, E>
    where
        U: UrlFor,
        E: From<Self::Error> + From<U::Error>,
    {
        let (note_url, from_url, to_urls, cc_urls) =
            futures::try_join!(
                urls.url_for_note(self.key()),
                async {
                    if let Some(ref from) = self.from {
                        Ok(Some(urls.url_for_account(&from).await?))
                    } else {
                        Ok(None)
                    }
                },
                FuturesOrdered::from_iter(self.to.iter().map(|r| r.url_for(urls)))
                    .try_collect::<Vec<_>>(),
                FuturesOrdered::from_iter(self.cc.iter().map(|r| r.url_for(urls)))
                    .try_collect::<Vec<_>>()
            )?;

        let mut note = object::Note::new();

        (|| {
            let o = &mut note.object_props;

            o.set_id(note_url)?;

            if let Some(created_at) = self.created_at.clone() {
                o.set_published(DateTime::<FixedOffset>::from(created_at))?;
            }

            if let Some(updated_at) = self.updated_at.clone() {
                o.set_updated(DateTime::<FixedOffset>::from(updated_at))?;
            }

            if let Some(from_url) = from_url {
                o.set_attributed_to_xsd_any_uri(from_url)?;
            }

            o.set_many_to_xsd_any_uris(to_urls)?;
            o.set_many_cc_xsd_any_uris(cc_urls)?;

            o.set_content_xsd_string(self.content.clone())?;

            Ok::<_, Self::Error>(())
        })()?;

        Ok(note)
    }
}

impl RemoteNoteInfo {
    pub fn new(uri: Url) -> RemoteNoteInfo {
        RemoteNoteInfo { uri }
    }
}
