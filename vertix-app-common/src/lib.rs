use std::{env, str::FromStr, path::PathBuf};
use url::Url;

mod urls;
pub use urls::Urls;

mod error;
pub use error::Error;
use error::Result;

pub mod helpers;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub domain: String,
    pub base_url: Url,
    pub trusted_certificate_files: Vec<PathBuf>,
}

impl Config {
    pub fn from_env() -> Result<Config> {
        let host: String = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());

        let port: u16 = match env::var("PORT") {
            Ok(port_str) => u16::from_str(&port_str)
                .map_err(|_| Error::InternalError(format!("Invalid port: {port_str}").into()))?,
            Err(_) => 8080
        };

        let domain = env::var("VERTIX_DOMAIN")
            .unwrap_or_else(|_| "localhost".into());

        if domain == "localhost" {
            log::warn!("VERTIX_DOMAIN is currently set to localhost. \
                This will not work properly. \
                Please set it to the domain name Vertix can be reached at.");
        }

        let base_url = Url::parse(&env::var("VERTIX_BASE_URL")
            .unwrap_or_else(|_| format!("http://{domain}:{port}/")))?;

        let trusted_certificate_files = env::var("VERTIX_TRUSTED_CERTS")
            .map(|value| value.split(",").map(|s| s.into()).collect())
            .unwrap_or_else(|_| vec![]);

        Ok(Config {
            host,
            port,
            domain,
            base_url,
            trusted_certificate_files,
        })
    }

    pub fn is_own_url(&self, url: &Url) -> bool {
        self.base_url.make_relative(url).is_some()
    }
}
