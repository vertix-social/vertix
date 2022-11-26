use actix_web::web;

mod webfinger;
mod account;
mod note;
mod interaction;
mod inbox;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.configure(webfinger::config);
    cfg.configure(account::config);
    cfg.configure(note::config);
    cfg.configure(interaction::config);
    cfg.configure(inbox::config);
}
