use std::{sync::Arc, borrow::Cow};

use aragog::Error as AragogError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Aragog error: {0}")]
    Aragog(#[source] Arc<AragogError>),

    #[error("ActivityStreams error: {0}")]
    ActivityStreams(#[source] crate::activitystreams::Error),

    #[error("Conversion failed: missing field: {0}")]
    ConversionMissingField(Cow<'static, str>),

    #[error("Not found: {model} ({params})")]
    NotFound {
        model: Cow<'static, str>,
        params: serde_json::Value,
    },
}

impl Error {
    pub fn is_not_found(&self) -> bool {
        match self {
            Error::Aragog(e) => match **e {
                AragogError::NotFound { .. } => true,
                _ => false
            },
            Error::NotFound { .. } => true,
            _ => false
        }
    }
    
    pub fn http_code(&self) -> u16 {
        match self {
            Error::Aragog(err) => err.http_code(),
            Error::NotFound { .. } => 404,
            _ => 500,
        }
    }
}

impl From<AragogError> for Error {
    fn from(err: AragogError) -> Self {
        Error::Aragog(Arc::new(err))
    }
}

impl From<Arc<AragogError>> for Error {
    fn from(err: Arc<AragogError>) -> Self {
        Error::Aragog(err)
    }
}

impl<T: Into<crate::activitystreams::Error>> From<T> for Error {
    fn from(err: T) -> Self {
        Error::ActivityStreams(err.into())
    }
}

#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, Error>;
