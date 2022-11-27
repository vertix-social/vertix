use std::{fs::File, io::Read};

use actix_webfinger::Webfinger;
use aragog::DatabaseAccess;
use lapin::Channel;
use reqwest::header;
use url::Url;
use regex::Regex;
use lazy_static::lazy_static;
use urlencoding::encode;
use vertix_model::{Account, Document};
use vertix_comm::{
    messages::{Action, ActionResponse},
    expect_reply_of
};

use crate::{error::Result, Error, Config};

pub async fn find_or_fetch_account_by_uri<D>(
    uri: &Url,
    config: &Config,
    db: &D,
    ch: &Channel,
) -> Result<Document<Account>>
where
    D: DatabaseAccess,
{
    // Handle own account url
    if config.is_own_url(uri) {
        let uri_path = uri.path();

        lazy_static! {
            static ref REGEX: Regex = Regex::new("/users/([^/]+)$").unwrap();
        }

        let username = REGEX.captures(uri_path)
            .ok_or(Error::InternalError("This is not a user URL".into()))?
            .get(1).unwrap()
            .as_str();

        let account = Account::find_by_username(username, None, &*db).await?;

        return Ok(account);
    }

    // Foreign account url
    match Account::find_by_uri(&uri, &*db).await {
        Ok(account) => Ok(account),
        Err(e) if e.is_not_found() => {
            let account = expect_reply_of!(
                Action::FetchAccount(uri.to_owned()).remote_call(&ch).await?;
                ActionResponse::FetchAccount(account) => account
            )?;

            Ok(account)
        },
        Err(e) => Err(e.into())
    }
}

pub async fn webfinger(
    client: &reqwest::Client,
    scheme: &str,
    username: &str,
    domain: &str,
    https: bool
) -> Result<Webfinger> {
    let mut url = Url::parse("http://example.com/.well-known/webfinger")?;

    url.set_scheme(if https { "https" } else { "http" }).unwrap();
    url.set_host(Some(domain))?;
    url.set_query(Some(&format!("resource={}:{}@{}",
        encode(scheme), encode(username), encode(domain))));

    let result = async {
        client.get(url)
            .header(header::ACCEPT, "application/json")
            .send().await?
            .error_for_status()?
            .json().await
    }.await;

    result.map_err(Error::WebfingerFetch)
}

pub fn build_reqwest_client(config: &Config) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .user_agent(concat!("vertix/", env!("CARGO_PKG_VERSION")));

    for path in &config.trusted_certificate_files {
        let mut file = File::open(path)?;
        let mut buf = vec![];
        file.read_to_end(&mut buf)?;
        let cert = reqwest::Certificate::from_pem(&buf)?;
        builder = builder.add_root_certificate(cert);
    }

    Ok(builder.build()?)
}
