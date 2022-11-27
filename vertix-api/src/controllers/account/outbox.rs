use std::sync::Arc;

use activitystreams::activity;
use actix_web::{web, get, Responder};
use futures::{stream::FuturesOrdered, TryStreamExt};
use vertix_model::{
    Account,
    PageLimit,
    activitystreams::{
        ToObject,
        UrlFor,
        make_actor_and_object_activity,
        make_ordered_collection,
        make_ordered_collection_page
    },
};

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

    let collection = make_ordered_collection(
        urls.url_for_account_outbox(account.key()).await?,
        urls.url_for_account_outbox_page(account.key(), 1).await?,
        Account::count_published_notes(&account, &*db).await?
    )?;

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

    let notes = urls.note_cache.put_many(
        Account::get_published_notes(&account, page_limit, &*db).await?).await;

    // Output should be a collection page of Create/Note
    let items: Vec<_> = FuturesOrdered::from_iter(
        notes.iter().map(|note| async {
            let object = note.to_object::<_, crate::Error>(&urls).await?;
            make_actor_and_object_activity::<activity::Create, _>(object)
                .map_err(crate::Error::from)
        })
    ).try_collect().await?;

    let col_page = make_ordered_collection_page(
        Some(page_limit),
        urls.url_for_account_outbox(account.key()).await?,
        Some(urls.url_for_account_outbox_page(account.key(), page + 1).await?),
        items
    )?;

    Ok(ActivityJson(col_page))
}
