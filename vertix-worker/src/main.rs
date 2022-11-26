use anyhow::Result;
use log::info;
use vertix_app_common::Config;
use std::{future::Future, sync::Arc};
use futures::stream::StreamExt;
use lapin::Channel;

use vertix_comm::*;
use vertix_model::AragogConnectionManager;

mod log_test_announce;
mod process_transaction;
mod log_interaction;
mod send_interactions_to_remote;
mod receive_activities;
mod deliver_activities;

#[actix_rt::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("info")
    ).init();

    info!("{} {} initializing", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let config = Arc::new(Config::from_env()?);

    let conn = vertix_comm::create_connection().await?;

    {
        let init_channel = conn.create_channel().await?;
        messages::setup(&init_channel).await?;
    }

    let pool = bb8::Pool::builder().build(AragogConnectionManager).await?;

    let client = reqwest::Client::builder()
        .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
        .build()?;

    macro_rules! start {
        ($function:expr, $($ident:ident = $arg:expr),*) => ({
            let channel = conn.create_channel().await?;
            $(let $ident = $arg;)*
            actix_rt::spawn(async move { $function(&channel, $($ident),*).await });
        });
        ($function:expr) => (start!($function,));
    }

    start!(log_test_announce::listen);
    start!(process_transaction::listen, pool = pool.clone(), client = client.clone());
    start!(log_interaction::listen);
    start!(send_interactions_to_remote::listen, config = config.clone(), pool = pool.clone());
    start!(receive_activities::listen, config = config.clone(), pool = pool.clone());
    start!(deliver_activities::listen, client = client.clone());

    info!("ready.");

    actix_rt::signal::ctrl_c().await?;

    Ok(())
}

/// Helper for commonly used pattern when processing incoming messages
pub(crate) async fn process_queue<T, Fut>(
    ch: &Channel,
    queue_name: &str,
    mut process: impl FnMut(T, Arc<Delivery<()>>) -> Fut
) -> Result<()>
where
    T: ReceiveMessage,
    Fut: Future<Output=Result<()>>,
{
    ch.basic_qos(2, Default::default()).await?;

    let mut stream = <T as ReceiveMessage>::receive(ch, queue_name, Default::default()).await?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => {
                let (message, data) = message.split_data();
                let message = Arc::new(message);

                if let Err(err) = process(data, message.clone()).await {
                    log::warn!("Error while processing from {queue_name}: {err}");
                    message.nack().await?;
                } else {
                    message.ack().await?;
                }
            },
            Err(err) => {
                log::warn!("Error in {queue_name}: {err}");
            }
        }
    }

    Ok(())
}
