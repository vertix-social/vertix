use aragog::{DatabaseAccess, DatabaseRecord};
use lapin::Channel;
use url::Url;
use vertix_model::Account;
use vertix_comm::messages::{Transaction, Action, ActionResponse};
use vertix_comm::RpcMessage;

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
        let uri_string = uri.to_string();
        //let regex = Regex::new("/users/[^/]+/");
        todo!();
    }

    match Account::find_by_uri(&uri, &*db).await {
        Ok(account) => Ok(account),
        Err(e) if e.is_not_found() => {
            let mut reply = Transaction { actions: vec![
                Action::FetchAccount(uri.to_owned()),
            ] }.remote_call(&ch).await?;

            let account = match reply.responses.pop() {
                Some(ActionResponse::FetchAccount(account)) => account,
                _ => return Err(Error::InternalError("wrong response type for action".into()))
            };

            Ok(account)
        },
        Err(e) => Err(e.into())
    }
}
