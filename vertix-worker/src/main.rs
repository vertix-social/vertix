use anyhow::Result;
use lapin::Channel;
use log::{info, warn};
use futures::stream::StreamExt;

use vertix_comm::*;
use vertix_comm::messages::TestAnnounce;

#[actix_rt::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("info")
    ).init();

    info!("{} {} initializing", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let conn = vertix_comm::create_connection().await?;

    {
        let init_channel = conn.create_channel().await?;
        messages::setup(&init_channel).await?;
    }

    macro_rules! start {
        ($function:ident) => ({
            let channel = conn.create_channel().await?;
            actix_rt::spawn(async move { $function(&channel).await });
        })
    }

    start!(log_test_announce);

    info!("ready.");

    actix_rt::signal::ctrl_c().await?;

    Ok(())
}

async fn log_test_announce(channel: &Channel) -> Result<()> {
    let mut stream = TestAnnounce::receive_copies(channel, "").await?;

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
