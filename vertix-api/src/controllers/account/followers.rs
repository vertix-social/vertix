use std::sync::Arc;

use aragog::Record;
use actix_web::{web, get, Responder, http::StatusCode, put};
use futures::{stream::FuturesOrdered, TryStreamExt};
use serde_json::json;
use vertix_comm::{messages::{Action, ActionResponse}, expect_reply_of};
use vertix_model::{
    Account,
    PageLimit,
    Follow,
    activitystreams::{
        ToObject,
        UrlFor,
        make_ordered_collection,
        make_ordered_collection_page
    },
    Wrap,
};

use crate::{ApiState, error::Result, formats::ActivityJson, Error};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_account_followers);
    cfg.service(get_account_followers_page);
    cfg.service(get_account_following);
    cfg.service(get_account_following_page);
    cfg.service(initiate_follow);
    cfg.service(list_pending_followers);
    cfg.service(accept_follow);
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
        followers.iter().map(|person| person.to_object::<_, Error>(&urls))
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
        following.iter().map(|person| person.to_object::<_, Error>(&urls))
    ).try_collect().await?;

    let col_page = make_ordered_collection_page(
        Some(page_limit),
        urls.url_for_account_following(account.key()).await?,
        Some(urls.url_for_account_following_page(account.key(), page + 1).await?),
        items
    )?;

    Ok(ActivityJson(col_page))
}

#[put("/api/v1/accounts/{from}/following/accounts/{to}")]
pub async fn initiate_follow(
    keys: web::Path<(String, String)>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let (from, to) = keys.into_inner();

    let ch = state.broker.create_channel().await?;

    let (created, follow) = expect_reply_of!(
        Action::InitiateFollow {
            from_account: from,
            to_account: to,
            uri: None,
        }.remote_call(&ch).await?;
        ActionResponse::InitiateFollow { created, follow } => (created, follow)
    )?;

    Ok((web::Json(follow), if created { StatusCode::CREATED } else { StatusCode::OK }))
}

#[get("/api/v1/accounts/{key}/followers/pending")]
pub async fn list_pending_followers(
    key: web::Path<String>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let db = state.pool.get().await?;

    let account = Account::find(key.as_str(), &*db).await?.wrap();

    let pending = Follow::find_pending_to(&account, &*db).await?;

    Ok(web::Json(json!({
        "follows": pending
    })))
}

#[put("/api/v1/follows/{key}/accept")]
pub async fn accept_follow(
    key: web::Path<String>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let ch = state.broker.create_channel().await?;

    let (modified, follow) = expect_reply_of!(
        Action::SetFollowAccepted { key: key.into_inner(), accepted: true }.remote_call(&ch).await?;
        ActionResponse::SetFollowAccepted { modified, follow } => (modified, follow)
    )?;

    if modified {
        Ok((web::Json(follow), StatusCode::CREATED))
    } else if follow.accepted == Some(true) {
        Ok((web::Json(follow), StatusCode::OK))
    } else {
        Err(Error::Conflict("Follow has already been rejected.".into()))
    }
}
