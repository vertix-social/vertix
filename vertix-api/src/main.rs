use actix_web::{get, web, App, HttpServer, Responder};
use actix_web::middleware::Logger;
use bb8::ErrorSink;

use std::str::FromStr;
use serde_json::json;
use anyhow::Result;
use log::{info, warn};

use vertix_model::AragogConnectionManager;

mod controllers;
mod error;

pub use error::Error;

#[derive(Debug)]
pub struct ApiState {
    domain: String,
    pool: bb8::Pool<AragogConnectionManager>,
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

    let pool = bb8::Pool::builder()
        .error_sink(Box::new(LogErrorSink))
        .build(AragogConnectionManager)
        .await?;
    
    // Double check that we can connect to the db before starting
    {
        let _ = vertix_model::create_connection().await?;
    }
    
    let state = ApiState { domain, pool };

    serve(&host, port, state).await?;

    Ok(())
}
