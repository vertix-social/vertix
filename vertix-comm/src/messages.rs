use lapin::{Channel, ExchangeKind};
use serde::{Serialize, Deserialize};
use super::SingleExchangeMessage;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestAnnounce {
    pub message: String,
}

impl SingleExchangeMessage for TestAnnounce {
    fn exchange() -> &'static str { "TestAnnounce" }

    fn routing_key(&self) -> std::borrow::Cow<'_, str> {
        "".into()
    }
}

pub async fn setup(ch: &Channel) -> Result<()> {
    ch.exchange_declare("TestAnnounce", ExchangeKind::Fanout, Default::default(),
        Default::default()).await?;

    Ok(())
}
