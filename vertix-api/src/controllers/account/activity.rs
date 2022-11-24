use activitystreams::actor::Person;
use actix_web::{web, get, Responder};
use crate::{ApiState, error::Result, formats::ActivityJson};
use vertix_model::Account;
use chrono::{DateTime, FixedOffset};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_activity_stream);
}

#[get("/users/{username}")]
pub async fn get_activity_stream(
    username: web::Path<String>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let db = state.pool.get().await?;

    let account = Account::find_by_username(&*username, None, &*db).await?;

    let mut person = Person::full();
    
    let domain = &state.domain;

    {
        let o = &mut person.base.base.object_props;
        o.set_id(format!("http://{domain}/users/{username}"))?;
        o.set_context_xsd_any_uri(activitystreams::context())?;
        o.set_name_xsd_string(username.as_str())?;

        if let Some(created_at) = account.created_at.clone() {
            o.set_published(DateTime::<FixedOffset>::from(created_at))?;
        }
    }

    {
        let e = &mut person.extension;
        e.set_preferred_username(account.username.clone())?;
        e.set_inbox(format!("http://{domain}/users/{username}/inbox"))?;
        e.set_outbox(format!("http://{domain}/users/{username}/outbox"))?;
    }

    Ok(ActivityJson(person))
}
