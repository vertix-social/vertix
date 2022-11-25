use std::sync::Arc;

use activitystreams::{collection::{OrderedCollection, OrderedCollectionPage}, activity};
use actix_web::{web, get, Responder};
use futures::{stream::FuturesOrdered, TryStreamExt};
use vertix_model::{Account, PageLimit, activitystreams::{ToObject, make_actor_and_object_activity, UrlFor}};

use crate::{ApiState, error::Result, formats::ActivityJson};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_account_outbox);
    cfg.service(get_account_outbox_page);
}

#[get("/users/{username}/outbox")]
pub async fn get_account_outbox(
    username: web::Path<String>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let db = state.pool.get().await?;

    let urls = state.urls(&*db);

    let account: Arc<_> = Account::find_by_username(&*username, None, &*db).await?.into();
    urls.account_cache.put(account.clone()).await;

    let mut collection = OrderedCollection::new();

    collection.object_props.set_context_xsd_any_uri(activitystreams::context())?;
    collection.object_props.set_id(
        urls.url_for_account_outbox(account.key()).await?)?;
    collection.collection_props.set_first_xsd_any_uri(
        urls.url_for_account_outbox_page(account.key(), 1).await?)?;
    collection.collection_props.set_total_items(1)?;

    Ok(ActivityJson(collection))
}

#[get("/users/{username}/outbox/page/{page}")]
pub async fn get_account_outbox_page(
    params: web::Path<(String, u32)>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let (username, page) = params.into_inner();
    let page_limit = PageLimit { page, ..PageLimit::default() };

    let db = state.pool.get().await?;

    let urls = state.urls(&*db);

    let account: Arc<_> = Account::find_by_username(&*username, None, &*db).await?.into();
    urls.account_cache.put(account.clone()).await;

    let mut col_page = OrderedCollectionPage::new();

    col_page.object_props.set_context_xsd_any_uri(activitystreams::context())?;

    let notes = urls.note_cache.put_many(
        Account::get_published_notes(&account, page_limit, &*db).await?.0).await;

    // Output should be a collection page of Create/Note
    let items: Vec<_> = FuturesOrdered::from_iter(
        notes.iter().map(|note| async {
            let object = note.to_object::<_, crate::Error>(&urls).await?;
            make_actor_and_object_activity::<activity::Create, _>(object)
                .map_err(crate::Error::from)
        })
    ).try_collect().await?;

    col_page.collection_page_props.set_part_of_xsd_any_uri(
        urls.url_for_account_outbox(account.key()).await?)?;

    if items.len() >= page_limit.limit as usize {
        col_page.collection_page_props.set_next_xsd_any_uri(
            urls.url_for_account_outbox_page(account.key(), page + 1).await?)?;
    }

    col_page.collection_props.set_many_items_base_boxes(items)?;

    Ok(ActivityJson(col_page))
}
