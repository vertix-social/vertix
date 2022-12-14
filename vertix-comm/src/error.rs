use std::borrow::Cow;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("json error")]
    Json(#[from] serde_json::Error),

    #[error("lapin error: {0}")]
    Lapin(#[from] lapin::Error),

    #[error("env var not set: AMQP_ADDR")]
    AmqpConfigMissing,

    #[error("no reply to rpc call")]
    NoReply,

    #[error("reply was not as expected: {0}")]
    InvalidReply(Cow<'static, str>),
}

pub type Result<T> = std::result::Result<T, Error>;
