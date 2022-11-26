use anyhow::Result;
use log::info;

use vertix_comm::*;
use vertix_model::AragogConnectionManager;

mod log_test_announce;
mod process_transaction;
mod log_interaction;
mod send_interactions_to_remote;
mod receive_activities;

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

    let pool = bb8::Pool::builder().build(AragogConnectionManager).await?;

    macro_rules! start {
        ($function:expr, $($ident:ident = $arg:expr),*) => ({
            let channel = conn.create_channel().await?;
            $(let $ident = $arg;)*
            actix_rt::spawn(async move { $function(&channel, $($ident),*).await });
        });
        ($function:expr) => (start!($function,));
    }

    start!(log_test_announce::listen);
    start!(process_transaction::listen, pool = pool.clone());
    start!(log_interaction::listen);
    start!(send_interactions_to_remote::listen, pool = pool.clone());
    start!(receive_activities::listen, pool = pool.clone());

    info!("ready.");

    actix_rt::signal::ctrl_c().await?;

    Ok(())
}
