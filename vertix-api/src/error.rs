use core::convert::Infallible;
use std::{borrow::Cow, sync::Arc};

use actix_web::{error::ResponseError, http::StatusCode};
use activitystreams::primitives::{
    XsdFloatError,
    XsdAnyUriError,
    XsdDateTimeError,
    XsdDurationError,
    MimeMediaTypeError,
    XsdNonNegativeFloatError,
    XsdNonNegativeIntegerError,
};

use vertix_model::activitystreams::Error as ActivityStreamsError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Model(#[from] vertix_model::Error),

    #[error("{0}")]
    Comm(#[from] vertix_comm::Error),

    #[error("pool error: {0}")]
    Pool(#[from] bb8::RunError<vertix_model::Error>),

    #[error("activity streams validation error: {0}")]
    ActivityStreams(#[from] ActivityStreamsError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("url parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("webfinger error: {0}")]
    Webfinger(#[from] actix_webfinger::FetchError),

    #[error("internal error: {0}")]
    InternalError(Cow<'static, str>),

    #[error("not found")]
    NotFound,
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Model(ref err) => StatusCode::from_u16(err.http_code())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),

            // Assumption. Not sure if correct
            Error::Webfinger(_) => StatusCode::NOT_FOUND,

            Error::NotFound => StatusCode::NOT_FOUND,

            Error::Pool(_) |
            Error::Comm(_) |
            Error::ActivityStreams(_) |
            Error::Io(_) |
            Error::Json(_) |
            Error::UrlParse(_) |
            Error::InternalError(_) =>
                StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

macro_rules! impl_via {
    ($via:path | $($from:path),* $(,)?) => {
        $(
            impl From<$from> for Error {
                fn from(err: $from) -> Self {
                    <$via>::from(err).into()
                }
            }
        )*
    }
}

impl_via!(vertix_model::Error | aragog::Error, Arc<aragog::Error>);
impl_via!(vertix_comm::Error | lapin::Error);

impl From<Infallible> for Error {
    fn from(_err: Infallible) -> Self {
        unreachable!()
    }
}

impl_via!(ActivityStreamsError |
    XsdFloatError,
    XsdAnyUriError,
    XsdDateTimeError,
    XsdDurationError,
    MimeMediaTypeError,
    XsdNonNegativeFloatError,
    XsdNonNegativeIntegerError,
);

impl From<vertix_app_common::Error> for Error {
    fn from(err: vertix_app_common::Error) -> Self {
        use vertix_app_common::Error::*;
        match err {
            Model(m) => m.into(),
            Comm(c) => c.into(),
            UrlParse(u) => u.into(),
            InternalError(e) => Error::InternalError(e),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
