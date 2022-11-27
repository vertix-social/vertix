use actix_web::{get, web, App, HttpServer, Responder};
use actix_web::middleware::Logger;
use aragog::DatabaseAccess;
use bb8::ErrorSink;

use serde_json::json;
use anyhow::Result;
use log::{info, warn};

use vertix_app_common::helpers::build_reqwest_client;
use vertix_model::AragogConnectionManager;
use vertix_app_common::{Urls, Config};

mod formats;
mod controllers;
mod error;

pub use error::Error;

pub struct ApiState {
    config: Config,
    pool: bb8::Pool<AragogConnectionManager>,
    broker: lapin::Connection,
    reqwest: reqwest::Client,
}

impl ApiState {
    pub fn urls<'a, D>(&'a self, db: &'a D) -> Urls<'a, D>
        where D: DatabaseAccess,
    {
        Urls::new(&self.config.base_url, db)
    }

    pub async fn db(&self) -> Result<bb8::PooledConnection<AragogConnectionManager>, Error> {
        self.pool.get().await.map_err(|e| e.into())
    }
}

#[get("/")]
async fn hello() -> impl Responder {
    web::Json(json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn serve(state: web::Data<ApiState>) -> Result<()> {
    let host = state.config.host.as_str();
    let port = state.config.port;
    info!("Listening on [{host}]:{port}");

    let state2 = state.clone();

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(state2.clone())
            .service(hello)
            .configure(controllers::config)
    })
        .bind((host, port))?
        .run()
        .await?;

    Ok(())
}

#[derive(Debug)]
struct LogErrorSink;

impl ErrorSink<vertix_model::Error> for LogErrorSink {
    fn sink(&self, error: vertix_model::Error) {
        warn!("Error in connection pool: {}", error);
    }

    fn boxed_clone(&self) -> Box<dyn ErrorSink<vertix_model::Error>> {
        Box::new(LogErrorSink) as Box<dyn ErrorSink<_>>
    }
}

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("info")
    ).init();

    let config = Config::from_env()?;

    let pool = bb8::Pool::builder()
        .error_sink(Box::new(LogErrorSink))
        .build(AragogConnectionManager)
        .await?;
    
    // Double check that we can connect to the db before starting
    {
        let _ = vertix_model::create_connection().await?;
    }

    let broker = vertix_comm::create_connection().await?;

    let reqwest = build_reqwest_client(&config)?;

    let state = web::Data::new(ApiState { config, pool, broker, reqwest });

    serve(state).await?;

    Ok(())
}
