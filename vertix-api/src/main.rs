use actix_web::{get, web, App, HttpServer, Responder};
use actix_web::middleware::Logger;
use aragog::DatabaseAccess;
use bb8::ErrorSink;
use url::Url;

use std::str::FromStr;
use serde_json::json;
use anyhow::Result;
use log::{info, warn};

use vertix_model::AragogConnectionManager;

mod formats;
mod controllers;
mod urls;
mod error;

pub use error::Error;

#[derive(Debug)]
pub struct ApiState {
    domain: String,
    base_url: Url,
    pool: bb8::Pool<AragogConnectionManager>,
    broker: lapin::Connection,
}

impl ApiState {
    pub fn urls<'a, D>(&'a self, db: &'a D) -> crate::urls::Urls<'a, D>
        where D: DatabaseAccess,
    {
        crate::urls::Urls::new(&self.base_url, db)
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

async fn serve(host: &str, port: u16, state: ApiState) -> Result<()> {
    info!("Listening on [{host}]:{port}");
    
    let data = web::Data::new(state);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(data.clone())
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

    let host: String = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());

    let port: u16 = match std::env::var("PORT") {
        Ok(port_str) => u16::from_str(&port_str)?,
        Err(_) => 8080
    };

    let domain = std::env::var("VERTIX_DOMAIN")
        .unwrap_or_else(|_| "localhost".into());

    if domain == "localhost" {
        warn!("VERTIX_DOMAIN is currently set to localhost. \
            This will not work properly. \
            Please set it to the domain name Vertix can be reached at.");
    }

    let base_url = Url::parse(&std::env::var("VERTIX_BASE_URL")
        .unwrap_or_else(|_| format!("http://{domain}:{port}/")))?;

    let pool = bb8::Pool::builder()
        .error_sink(Box::new(LogErrorSink))
        .build(AragogConnectionManager)
        .await?;
    
    // Double check that we can connect to the db before starting
    {
        let _ = vertix_model::create_connection().await?;
    }

    let broker = vertix_comm::create_connection().await?;

    let state = ApiState { domain, base_url, pool, broker };

    serve(&host, port, state).await?;

    Ok(())
}
