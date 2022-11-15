use actix_web::web;

mod webfinger;
mod account;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.configure(webfinger::config);
    cfg.configure(account::config);
}
