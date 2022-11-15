use actix_web::web;

mod webfinger;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.configure(webfinger::config);
}
