use actix_web::{web, get, Responder};
use actix_webfinger::Webfinger;
use serde::Deserialize;
use vertix_comm::RpcMessage;
use vertix_comm::messages::{Transaction, Action, ActionResponse};
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
    let LookupQueryParams { username, domain } = query.into_inner();

    match Account::find_by_username(&username, domain.as_deref(), &*db).await {
        Ok(account) => Ok(web::Json(account)),
        Err(e) if e.is_not_found() && domain.is_some() => {
            // Fetch remote account
            let awc = crate::make_awc_client();

            let wf = Webfinger::fetch(&awc, None,
                &username,
                domain.as_deref().unwrap(),
                true).await?;

            let ch = state.broker.create_channel().await?;

            let url = wf.activitypub()
                .ok_or(Error::NotFound)?.href.as_deref()
                .ok_or(Error::NotFound)?.try_into()?;

            let mut reply = Transaction { actions: vec![
                Action::FetchAccount(url),
            ] }.remote_call(&ch).await?;

            let account = match reply.responses.pop() {
                Some(ActionResponse::FetchAccount(account)) => account,
                _ => return Err(Error::InternalError("wrong response type for action".into()))
            };

            Ok(web::Json(account))
        },
        Err(err) => Err(err.into())
    }
}
