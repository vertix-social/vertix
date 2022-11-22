use lapin::Channel;
use log::{info, warn, debug};
use anyhow::Result;

use vertix_comm::messages::TestAnnounce;
use vertix_comm::ReceiveMessage;

use futures::stream::StreamExt;

pub async fn listen(channel: &Channel) -> Result<()> {
    let mut stream = TestAnnounce::receive_copies(channel, "").await?;

    debug!("listening for TestAnnounce");

    while let Some(result) = stream.next().await {
        match result {
            Ok(msg) => {
                info!("TestAnnounce: {}", msg.message);
            },
            Err(err) => {
                warn!("error in TestAnnounce: {}", err);
            }
        }
    }

    Ok(())
}
