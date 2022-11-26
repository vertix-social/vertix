use actix_web::web;

pub mod fetch;
pub mod activity;
pub mod outbox;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.configure(fetch::config);
    cfg.configure(activity::config);
    cfg.configure(outbox::config);
}
