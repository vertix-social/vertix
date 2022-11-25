use actix_web::web;
use actix_webfinger::Webfinger;
use vertix_model::activitystreams::UrlFor;

use crate::ApiState;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

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
                    let urls = state.urls(&*db);
                    match Account::find_by_username(&account, None, &*db).await {
                        Ok(m) => {
                            let mut wf = Webfinger::new(&format!("{account}@{domain}"));
                            let m = Arc::new(m);
                            urls.account_cache.put(m.clone()).await;

                            let account_url = urls.url_for_account(m.key()).await?;
                            wf.add_activitypub(account_url.as_str());
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
