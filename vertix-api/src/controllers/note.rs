use actix_web::{web, get, post, Responder};
use aragog::Record;
use vertix_model::{Note, Account};
use vertix_comm::messages::{Transaction, Action, ActionResponse};
use vertix_comm::RpcMessage;
use serde::Deserialize;

use crate::Error;
use crate::{error::Result, ApiState};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_note);
    cfg.service(publish_note);
}

#[get("/api/v1/notes/{key}")]
pub async fn get_note(
    state: web::Data<ApiState>,
    path: web::Path<String>
) -> Result<impl Responder> {
    let db = state.pool.get().await?;

    let note = Note::find(&*path, &*db).await?;

    Ok(web::Json(note))
}

// FIXME auth
#[derive(Debug, Deserialize)]
pub struct PublishNoteQuery {
    pub from_username: String,
}

#[post("/api/v1/notes")]
pub async fn publish_note(
    state: web::Data<ApiState>,
    query: web::Query<PublishNoteQuery>,
    body: web::Json<Note>
) -> Result<impl Responder> {
    let ch = state.broker.create_channel().await?;
    let db = state.pool.get().await?;

    let account = Account::find_by_username(&query.from_username, None, &*db).await?;

    let transaction = Transaction { actions: vec![
        Action::PublishNote(Note {
            from: Some(account.key().into()),
            ..body.into_inner()
        })
    ] };

    let response = transaction.remote_call(&ch).await?;

    match &*response.responses {
        [ActionResponse::PublishNote(note)] => Ok(web::Json(note.clone())),
        _ => Err(Error::InternalError("bad reply from worker".into()))
    }
}
