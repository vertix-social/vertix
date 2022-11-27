use aragog::{DatabaseAccess, DatabaseRecord};
use lapin::Channel;
use url::Url;
use regex::Regex;
use lazy_static::lazy_static;
use vertix_model::Account;
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
) -> Result<DatabaseRecord<Account>>
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
