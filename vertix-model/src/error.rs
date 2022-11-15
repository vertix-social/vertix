use aragog::Error as AragogError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Aragog error: {0}")]
    Aragog(#[from] AragogError),
}

impl Error {
    pub fn is_not_found(&self) -> bool {
        match self {
            Error::Aragog(AragogError::NotFound { .. }) => true,
            _ => false
        }
    }
    
    pub fn http_code(&self) -> u16 {
        match self {
            Error::Aragog(err) => err.http_code(),
        }
    }
}
