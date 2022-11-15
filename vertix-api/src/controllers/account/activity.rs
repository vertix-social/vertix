use activitystreams::actor::Person;
use actix_web::{web, get, Responder};
use crate::{ApiState, Error};
use vertix_model::Account;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_activity_stream);
}

#[get("/users/{username}")]
pub async fn get_activity_stream(
    username: web::Path<String>,
    state: web::Data<ApiState>
) -> Result<impl Responder, Error> {
    let db = state.pool.get().await?;

    let account = Account::find_by_username(&*username, None, &*db).await?;

    let mut person = Person::full();
    
    let domain = &state.domain;

    person.extension.set_preferred_username(account.username.clone())?;
    person.extension.set_inbox(format!("http://{domain}/users/{username}/inbox"))?;
    person.extension.set_outbox(format!("http://{domain}/users/{username}/outbox"))?;

    Ok(web::Json(person))
}
