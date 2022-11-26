use lapin::Channel;
use log::{info, warn, debug};
use anyhow::Result;

use vertix_comm::messages::Interaction;

use futures::stream::StreamExt;

pub async fn listen(channel: &Channel) -> Result<()> {
    let mut stream = Interaction::listen(channel, &[] as &[&str], &[]).await?;

    debug!("listening for Interaction");

    while let Some(result) = stream.next().await {
        match result {
            Ok(interaction) => {
                info!("Interaction: {}", serde_json::to_string(&interaction).unwrap());
            },
            Err(err) => {
                warn!("error in Interaction: {}", err);
            }
        }
    }

    Ok(())
}