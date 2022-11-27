use std::{convert::Infallible, io};

use activitystreams::collection::{OrderedCollection, OrderedCollectionPage};
use async_trait::async_trait;
use url::Url;
use activitystreams::{primitives::*, BaseBox};
use activitystreams::object::properties::ObjectProperties;
use activitystreams::activity::properties::ActorAndObjectProperties;

use crate::PageLimit;

/// Resolves URLs to be used for various links within ActivityStreams documents
#[async_trait]
pub trait UrlFor {
    type Error: std::error::Error;

    async fn url_for_account(&self, key: &str) -> Result<Url, Self::Error>;
    async fn url_for_account_inbox(&self, key: &str) -> Result<Url, Self::Error>;
    async fn url_for_account_outbox(&self, key: &str) -> Result<Url, Self::Error>;
    async fn url_for_account_outbox_page(&self, key: &str, page: u32) -> Result<Url, Self::Error>;
    async fn url_for_account_followers(&self, key: &str) -> Result<Url, Self::Error>;
    async fn url_for_account_followers_page(&self, key: &str, page: u32) -> Result<Url, Self::Error>;
    async fn url_for_account_following(&self, key: &str) -> Result<Url, Self::Error>;
    async fn url_for_account_following_page(&self, key: &str, page: u32) -> Result<Url, Self::Error>;

    async fn url_for_note(&self, key: &str) -> Result<Url, Self::Error>;

    fn url_for_shared_inbox(&self) -> Result<Url, Self::Error>;
}

/// Generates an ActivityStreams model from the object
#[async_trait(?Send)]
pub trait ToObject {
    type Output: activitystreams::object::Object;
    type Error;

    async fn to_object<U, E>(&self, urls: &U) -> Result<Self::Output, E>
    where
        U: UrlFor,
        E: From<Self::Error> + From<U::Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    XsdFloat(#[from] XsdFloatError),
    #[error("{0}")]
    XsdAnyUri(#[from] XsdAnyUriError),
    #[error("{0}")]
    XsdDateTime(#[from] XsdDateTimeError),
    #[error("{0}")]
    XsdDuration(#[from] XsdDurationError),
    #[error("{0}")]
    MimeMediaType(#[from] MimeMediaTypeError),
    #[error("{0}")]
    XsdNonNegativeFloat(#[from] XsdNonNegativeFloatError),
    #[error("{0}")]
    XsdNonNegativeInteger(#[from] XsdNonNegativeIntegerError),
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("missing actor from attributed_at")]
    MissingActor,
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Error {
        unreachable!()
    }
}

pub fn make_actor_and_object_activity<A, O>(object: O) -> Result<A, Error>
where
    A: Default + AsMut<ActorAndObjectProperties>,
    O: activitystreams::Object + AsRef<ObjectProperties> + TryInto<BaseBox, Error=io::Error>,
{
    let mut activity = A::default();

    let actor_url = object.as_ref().get_attributed_to_xsd_any_uri().cloned()
        .ok_or(Error::MissingActor)?;

    activity.as_mut().set_object_base_box(object)?;
    activity.as_mut().set_actor_xsd_any_uri(actor_url)?;

    Ok(activity)
}

pub fn make_ordered_collection(
    self_url: Url,
    first_page_url: Url,
    total_count: u64,
) -> Result<OrderedCollection, Error> {
    let mut collection = OrderedCollection::new();

    collection.object_props.set_context_xsd_any_uri(activitystreams::context())?;
    collection.object_props.set_id(self_url)?;
    collection.collection_props.set_first_xsd_any_uri(first_page_url)?;
    collection.collection_props.set_total_items(total_count)?;

    Ok(collection)
}

pub fn make_ordered_collection_page<T>(
    page_limit: Option<PageLimit>,
    collection_url: Url,
    next_page_url: Option<Url>,
    items: Vec<T>,
) -> Result<OrderedCollectionPage, Error>
where
    T: TryInto<BaseBox>,
    Error: From<<T as TryInto<BaseBox>>::Error>,
{
    let mut col_page = OrderedCollectionPage::new();

    col_page.object_props.set_context_xsd_any_uri(activitystreams::context())?;

    col_page.collection_page_props.set_part_of_xsd_any_uri(collection_url)?;

    match (page_limit, next_page_url) {
        (Some(page_limit), _) if page_limit.limit as usize > items.len() => {
            // Assume no next page
        },
        (_, Some(next_page_url)) => {
            col_page.collection_page_props.set_next_xsd_any_uri(next_page_url)?;
        },
        _ => ()
    }

    col_page.collection_props.set_many_items_base_boxes(items)?;

    Ok(col_page)
}
