use std::sync::Arc;

use aragog::DatabaseConnection;
use lapin::Channel;
use vertix_app_common::{helpers, Config};
use vertix_comm::SendMessage;
use vertix_comm::messages::{ReceiveActivity, Transaction, Action};
use vertix_model::AragogConnectionManager;
use anyhow::{bail, anyhow, Result};
use url::Url;
use activitystreams::activity;

use crate::process_queue;

pub async fn listen(ch: &Channel, config: Arc<Config>, pool: bb8::Pool<AragogConnectionManager>) -> Result<()> {
    log::debug!("Listening for ReceiveActivity");

    process_queue(ch, "ReceiveActivity.process", |data, _| process(data, &*config, ch, &pool)).await
}

async fn process(
    receive_activity: ReceiveActivity,
    config: &Config,
    ch: &Channel,
    pool: &bb8::Pool<AragogConnectionManager>
) -> Result<()> {
    let db = pool.get().await?;

    let activity = receive_activity.activity;

    match activity.kind() {
        Some("Follow") => process_follow(activity.into_concrete()?, config, ch, &*db).await?,
        _ => bail!("Unprocessable activity: {activity:?}")
    }

    Ok(())
}

async fn process_follow(
    activity: activity::Follow,
    config: &Config,
    ch: &Channel,
    db: &DatabaseConnection,
) -> Result<()> {
    log::debug!("Process remote follow {activity:?}");

    let actor_uri: Url = activity.follow_props.get_actor_xsd_any_uri()
        .ok_or_else(|| anyhow!("Follow actor is not URI"))?
        .as_url().clone();

    let object_uri: Url = activity.follow_props.get_object_xsd_any_uri()
        .ok_or_else(|| anyhow!("Follow object is not URI"))?
        .as_url().clone();

    let from = helpers::find_or_fetch_account_by_uri(
        &actor_uri, &config, db, ch).await?;
    let to = helpers::find_or_fetch_account_by_uri(
        &object_uri, &config, db, ch).await?;

    if !(from.is_remote() && to.is_remote()) {
        Transaction { actions: vec![
            Action::InitiateFollow {
                from_account: from.key().to_owned(),
                to_account: to.key().to_owned(),
            }
        ] }.send(ch).await?;
    } else {
        log::warn!("Tried to process follow involving two remote parties")
    }

    Ok(())
}
