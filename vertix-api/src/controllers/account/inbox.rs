use actix_web::{web, post, Responder, http::StatusCode};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(post_account_inbox);
}

#[post("/users/{username}/inbox")]
pub async fn post_account_inbox(username: web::Path<String>) -> impl Responder {
    ("Not yet implemented", StatusCode::NOT_IMPLEMENTED)
}
