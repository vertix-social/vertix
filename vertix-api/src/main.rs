use actix_web::{get, web, App, HttpServer, Responder};
use actix_web::middleware::Logger;

use serde_json::json;
use anyhow::Result;
use std::str::FromStr;
use log::info;

#[get("/")]
async fn hello() -> impl Responder {
    web::Json(json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn serve(host: &str, port: u16) -> Result<()> {

    info!("Listening on [{host}]:{port}");

    HttpServer::new(|| {
        App::new()
            .service(hello)
            .wrap(Logger::default())
    })
        .bind((host, port))?
        .run()
        .await?;

    Ok(())
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

    serve(&host, port).await?;

    Ok(())
}
