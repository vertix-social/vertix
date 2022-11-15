use core::convert::Infallible;

use vertix_model::Error as ModelError;
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
    Model(#[from] ModelError),

    #[error("pool error: {0}")]
    Pool(#[from] bb8::RunError<ModelError>),

    #[error("activity streams validation error: {0}")]
    ActivityStreams(ActivityStreamsError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Model(ref err) => StatusCode::from_u16(err.http_code())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),

            Error::Pool(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::ActivityStreams(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<aragog::Error> for Error {
    fn from(err: aragog::Error) -> Self {
        ModelError::from(err).into()
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
