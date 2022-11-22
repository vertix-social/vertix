use anyhow::Result;
use log::info;

use vertix_comm::*;
use vertix_model::AragogConnectionManager;

mod log_test_announce;
mod process_transaction;
mod log_interaction;

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
        ($function:expr, $($arg:expr),*) => ({
            let channel = conn.create_channel().await?;
            actix_rt::spawn(async move { $function(&channel, $($arg),*).await });
        });
        ($function:expr) => (start!($function,));
    }

    start!(log_test_announce::listen);
    start!(process_transaction::listen, pool.clone());
    start!(log_interaction::listen);

    info!("ready.");

    actix_rt::signal::ctrl_c().await?;

    Ok(())
}
