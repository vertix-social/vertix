use actix_web::{web, get, Responder};
use actix_webfinger::Webfinger;
use serde::Deserialize;
use vertix_comm::{expect_reply_of, messages::{Action, ActionResponse}};
use vertix_model::Account;

use crate::{ApiState, Error};
use crate::error::Result;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(lookup_account);
}

#[derive(Deserialize)]
struct LookupQueryParams {
    username: String,
    domain: Option<String>,
}

#[get("/api/v1/accounts/lookup")]
async fn lookup_account(
    state: web::Data<ApiState>,
    query: web::Query<LookupQueryParams>,
) -> Result<impl Responder> {
    let db = state.pool.get().await?;
    let LookupQueryParams { username, mut domain } = query.into_inner();

    // Handle own domain
    if domain.as_deref() == Some(&state.config.domain) {
        domain = None;
    }

    match Account::find_by_username(&username, domain.as_deref(), &*db).await {
        Ok(account) => Ok(web::Json(account)),
        Err(e) if e.is_not_found() && domain.is_some() => {
            // Fetch remote account
            let awc = crate::make_awc_client();

            let wf = Webfinger::fetch(&awc,
                Some("acct:"),
                &username,
                domain.as_deref().unwrap(),
                true).await?;

            let ch = state.broker.create_channel().await?;

            let url = wf.activitypub()
                .ok_or(Error::NotFound)?.href.as_deref()
                .ok_or(Error::NotFound)?.try_into()?;

            let account = expect_reply_of!(
                Action::FetchAccount(url).remote_call(&ch).await?;
                ActionResponse::FetchAccount(account) => account
            )?;

            Ok(web::Json(account))
        },
        Err(err) => Err(err.into())
    }
}
