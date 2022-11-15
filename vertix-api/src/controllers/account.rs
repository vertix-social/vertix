use actix_web::web;

pub mod activity;
pub mod inbox;
pub mod outbox;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.configure(activity::config);
    cfg.configure(inbox::config);
    cfg.configure(outbox::config);
}
