use std::{borrow::Cow, convert::Infallible, sync::Arc};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Model(#[from] vertix_model::Error),

    #[error("{0}")]
    Comm(#[from] vertix_comm::Error),

    #[error("url parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("{0}")]
    InternalError(Cow<'static, str>),
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl From<aragog::Error> for Error {
    fn from(err: aragog::Error) -> Self {
        Error::Model(err.into())
    }
}

impl From<Arc<aragog::Error>> for Error {
    fn from(err: Arc<aragog::Error>) -> Self {
        Error::Model(err.into())
    }
}
