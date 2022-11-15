use vertix_model::Error as ModelError;

use actix_web::{error::ResponseError, http::StatusCode};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Model(#[from] ModelError),

    #[error("pool error: {0}")]
    Pool(#[from] bb8::RunError<ModelError>)
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Model(ref err) => StatusCode::from_u16(err.http_code())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),

            Error::Pool(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<aragog::Error> for Error {
    fn from(err: aragog::Error) -> Self {
        ModelError::from(err).into()
    }
}
