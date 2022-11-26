use actix_web::{web, routes, Responder, http::StatusCode};
use activitystreams::activity::ActivityBox;
use serde::Deserialize;
use vertix_comm::messages::ReceiveActivity;
use vertix_comm::SendMessage;

use crate::{ApiState, error::Result};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(post_inbox);
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct PathData {
    username: Option<String>,
}

#[routes]
#[post("/users/{username}/inbox")]
#[post("/inbox")]
pub async fn post_inbox(
    state: web::Data<ApiState>,
    _path_data: web::Path<PathData>,
    content: web::Json<ActivityBox>,
) -> Result<impl Responder> {
    let ch = state.broker.create_channel().await?;

    ReceiveActivity {
        activity: content.into_inner(),
    }.send(&ch).await?;

    Ok(("", StatusCode::CREATED))
}
