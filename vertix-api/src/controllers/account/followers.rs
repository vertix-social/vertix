use std::sync::Arc;

use actix_web::{web, get, Responder};
use futures::{stream::FuturesOrdered, TryStreamExt};
use vertix_model::{
    Account,
    PageLimit,
    activitystreams::{
        ToObject,
        UrlFor,
        make_ordered_collection,
        make_ordered_collection_page
    },
};

use crate::{ApiState, error::Result, formats::ActivityJson};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_account_followers);
    cfg.service(get_account_followers_page);
    cfg.service(get_account_following);
    cfg.service(get_account_following_page);
}

#[get("/users/{username}/followers")]
pub async fn get_account_followers(
    username: web::Path<String>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let db = state.pool.get().await?;

    let urls = state.urls(&*db);

    let account: Arc<_> = Account::find_by_username(&*username, None, &*db).await?.into();
    urls.account_cache.put(account.clone()).await;

    let collection = make_ordered_collection(
        urls.url_for_account_followers(account.key()).await?,
        urls.url_for_account_followers_page(account.key(), 1).await?,
        Account::count_followers(&account, &*db).await?
    )?;

    Ok(ActivityJson(collection))
}

#[get("/users/{username}/followers/page/{page}")]
pub async fn get_account_followers_page(
    params: web::Path<(String, u32)>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let (username, page) = params.into_inner();
    let page_limit = PageLimit { page, ..PageLimit::default() };

    let db = state.pool.get().await?;

    let urls = state.urls(&*db);

    let account: Arc<_> = Account::find_by_username(&*username, None, &*db).await?.into();
    urls.account_cache.put(account.clone()).await;

    let followers = urls.account_cache.put_many(
        Account::get_followers(&account, page_limit, &*db).await?).await;

    // Output should be a collection page of Person
    let items: Vec<_> = FuturesOrdered::from_iter(
        followers.iter().map(|person| person.to_object::<_, crate::Error>(&urls))
    ).try_collect().await?;

    let col_page = make_ordered_collection_page(
        Some(page_limit),
        urls.url_for_account_followers(account.key()).await?,
        Some(urls.url_for_account_followers_page(account.key(), page + 1).await?),
        items
    )?;

    Ok(ActivityJson(col_page))
}

#[get("/users/{username}/following")]
pub async fn get_account_following(
    username: web::Path<String>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let db = state.pool.get().await?;

    let urls = state.urls(&*db);

    let account: Arc<_> = Account::find_by_username(&*username, None, &*db).await?.into();
    urls.account_cache.put(account.clone()).await;

    let collection = make_ordered_collection(
        urls.url_for_account_following(account.key()).await?,
        urls.url_for_account_following_page(account.key(), 1).await?,
        Account::count_following(&account, &*db).await?
    )?;

    Ok(ActivityJson(collection))
}

#[get("/users/{username}/following/page/{page}")]
pub async fn get_account_following_page(
    params: web::Path<(String, u32)>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let (username, page) = params.into_inner();
    let page_limit = PageLimit { page, ..PageLimit::default() };

    let db = state.pool.get().await?;

    let urls = state.urls(&*db);

    let account: Arc<_> = Account::find_by_username(&*username, None, &*db).await?.into();
    urls.account_cache.put(account.clone()).await;

    let following = urls.account_cache.put_many(
        Account::get_following(&account, page_limit, &*db).await?).await;

    // Output should be a collection page of Person
    let items: Vec<_> = FuturesOrdered::from_iter(
        following.iter().map(|person| person.to_object::<_, crate::Error>(&urls))
    ).try_collect().await?;

    let col_page = make_ordered_collection_page(
        Some(page_limit),
        urls.url_for_account_following(account.key()).await?,
        Some(urls.url_for_account_following_page(account.key(), page + 1).await?),
        items
    )?;

    Ok(ActivityJson(col_page))
}
