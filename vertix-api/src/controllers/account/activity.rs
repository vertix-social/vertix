use std::sync::Arc;

use actix_web::{web, get, Responder};
use crate::{ApiState, error::Result, formats::ActivityJson};
use vertix_model::{Account, activitystreams::ToObject};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_activity_stream);
}

#[get("/users/{username}")]
pub async fn get_activity_stream(
    username: web::Path<String>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let db = state.pool.get().await?;

    let urls = state.urls(&*db);

    let account: Arc<_> = Account::find_by_username(&*username, None, &*db).await?.into();

    let person = account.to_object::<_, crate::Error>(&urls).await?;

    Ok(ActivityJson(person))
}
