use actix_web::web;
use actix_webfinger::Webfinger;

use crate::ApiState;

use std::future::Future;
use std::pin::Pin;

use vertix_model::Account;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(actix_webfinger::resource::<VertixResolver>());
}

pub struct VertixResolver;

impl actix_webfinger::Resolver for VertixResolver {
    type State = web::Data<ApiState>;
    type Error = crate::Error;

    fn find(
        scheme: Option<&str>,
        account: &str,
        domain: &str,
        state: Self::State,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Webfinger>, Self::Error>>>> {
        if scheme == Some("acct:") && domain == state.domain {
            let account = account.to_owned();
            let domain = domain.to_owned();

            Box::pin(async move {
                    let db = state.pool.get().await?;
                    match Account::find_by_username(&account, None, &*db).await {
                        Ok(_) => {
                            let mut wf = Webfinger::new(&format!("{account}@{domain}"));
                            wf.add_activitypub(&format!("http://{domain}/users/{account}"));
                            Ok(Some(wf))
                        },
                        Err(err) if err.is_not_found() => {
                            Ok(None)
                        },
                        Err(err) => Err(err.into())
                    }
            })
        } else {
            Box::pin(async move { Ok(None) })
        }
    }

}
