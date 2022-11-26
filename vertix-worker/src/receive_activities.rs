use aragog::DatabaseConnection;
use lapin::Channel;
use vertix_comm::{messages::ReceiveActivity, ReceiveMessage};
use vertix_model::AragogConnectionManager;
use anyhow::{bail, Result};
use futures::stream::StreamExt;
use activitystreams::activity;

pub async fn listen(ch: &Channel, pool: bb8::Pool<AragogConnectionManager>) -> Result<()> {
    ch.basic_qos(2, Default::default()).await?;

    log::debug!("Listening for ReceiveActivity");

    let mut stream = ReceiveActivity::receive(
        ch, "ReceiveActivity.process", Default::default()).await?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => {
                if let Err(err) = process(message.data(), &pool).await {
                    log::warn!("Error while processing activity: {err}");
                    message.nack().await?;
                } else {
                    message.ack().await?;
                }
            },
            Err(err) => {
                log::warn!("Error in ReceiveActivity: {err}");
            }
        }
    }

    Ok(())
}

async fn process(
    receive_activity: &ReceiveActivity,
    pool: &bb8::Pool<AragogConnectionManager>
) -> Result<()> {
    let db = pool.get().await?;

    let activity = receive_activity.activity.clone();

    match activity.kind() {
        Some("Follow") => process_follow(&*db, activity.into_concrete()?).await?,
        _ => bail!("Unprocessable activity: {activity:?}")
    }

    Ok(())
}

async fn process_follow(
    db: &DatabaseConnection,
    activity: activity::Follow,
) -> Result<()> {
    log::warn!("TODO: process follow {activity:?}");
    Ok(())
}
