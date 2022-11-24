use actix_web::body::EitherBody;
use actix_web::{web, Responder, HttpResponse};
use actix_web::http::header::CONTENT_TYPE;

#[allow(dead_code)]
pub const ACTIVITYSTREAMS_CONTENT_TYPE: &str =
    "application/activity+json";

#[allow(dead_code)]
pub const ACTIVITYSTREAMS_LD_CONTENT_TYPE: &str =
    "application/ld+json; profile = \"https://www.w3.org/ns/activitystreams\"";

#[derive(Debug, Clone)]
pub struct ActivityJson<T>(pub T);

impl<T> Responder for ActivityJson<T>
    where web::Json<T>: Responder,
{
    type Body = EitherBody<<web::Json<T> as Responder>::Body>;

    fn respond_to(self, req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        web::Json(self.0)
            .customize()
            .insert_header((CONTENT_TYPE, ACTIVITYSTREAMS_CONTENT_TYPE))
            .respond_to(req)
    }
}
