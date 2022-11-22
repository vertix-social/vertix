use core::convert::Infallible;
use std::borrow::Cow;

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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Model(#[from] vertix_model::Error),

    #[error("{0}")]
    Comm(#[from] vertix_comm::Error),

    #[error("pool error: {0}")]
    Pool(#[from] bb8::RunError<vertix_model::Error>),

    #[error("activity streams validation error: {0}")]
    ActivityStreams(ActivityStreamsError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("internal error: {0}")]
    InternalError(Cow<'static, str>),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Model(ref err) => StatusCode::from_u16(err.http_code())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),

            Error::Pool(_) |
            Error::Comm(_) |
            Error::ActivityStreams(_) |
            Error::Io(_) |
            Error::InternalError(_) =>
                StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<aragog::Error> for Error {
    fn from(err: aragog::Error) -> Self {
        vertix_model::Error::from(err).into()
    }
}

impl From<lapin::Error> for Error {
    fn from(err: lapin::Error) -> Self {
        vertix_comm::Error::from(err).into()
    }
}

impl From<Infallible> for Error {
    fn from(_err: Infallible) -> Self {
        unreachable!()
    }
}

impl<T> From<T> for Error where ActivityStreamsError: From<T> {
    fn from(err: T) -> Self {
        ActivityStreamsError::from(err).into()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ActivityStreamsError{
    #[error("{0}")]
    XsdFloat(#[from] XsdFloatError),
    #[error("{0}")]
    XsdAnyUri(#[from] XsdAnyUriError),
    #[error("{0}")]
    XsdDateTime(#[from] XsdDateTimeError),
    #[error("{0}")]
    XsdDurationError(#[from] XsdDurationError),
    #[error("{0}")]
    MimeMediaTypeError(#[from] MimeMediaTypeError),
    #[error("{0}")]
    XsdNonNegativeFloatError(#[from] XsdNonNegativeFloatError),
    #[error("{0}")]
    XsdNonNegativeIntegerError(#[from] XsdNonNegativeIntegerError),
}

pub type Result<T> = std::result::Result<T, Error>;
