use actix_web::web;

mod webfinger;
mod account;
mod note;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.configure(webfinger::config);
    cfg.configure(account::config);
    cfg.configure(note::config);
}
