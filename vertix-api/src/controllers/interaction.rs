use std::time::Duration;

use actix_web::{web, get, Responder};
use actix_web_lab::sse;
use vertix_model::Recipient;
use vertix_comm::messages::Interaction;
use serde::Deserialize;
use futures::stream::TryStreamExt;

use crate::{error::Result, ApiState};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_interactions_stream);
}

#[derive(Debug, Deserialize)]
pub struct GetInteractionsStreamQuery {
    #[serde(default)]
    from: Option<String>,

    #[serde(default)]
    to: Option<String>,
}

impl GetInteractionsStreamQuery {
    pub fn from(&self) -> Vec<String> {
        if let Some(from_comma_sep) = self.from.as_ref() {
            from_comma_sep.split(",").map(|s| s.into()).collect()
        } else {
            vec![]
        }
    }

    pub fn to(&self) -> Vec<Recipient> {
        if let Some(to_comma_sep) = self.to.as_ref() {
            to_comma_sep.split(",").map(|s| match s {
                "public" => Recipient::Public,
                other => Recipient::Account(other.into())
            }).collect()
        } else {
            vec![]
        }
    }
}

#[get("/api/v1/interactions.stream")]
pub async fn get_interactions_stream(
    state: web::Data<ApiState>,
    query: web::Query<GetInteractionsStreamQuery>,
) -> Result<impl Responder> {
    let ch = state.broker.create_channel().await?;

    let stream = Interaction::listen(&ch, &query.from(), &query.to()).await?;

    Ok(sse::Sse::from_stream(stream.and_then(|item| async {
        Ok(sse::Data::new_json(item)?.into())
    })).with_keep_alive(Duration::from_secs(20)))
}
