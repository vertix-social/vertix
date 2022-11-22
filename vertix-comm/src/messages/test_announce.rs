use serde::{Serialize, Deserialize};

use lapin::{Channel, ExchangeKind, options::ExchangeDeclareOptions};

use crate::SingleExchangeMessage;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestAnnounce {
    pub message: String,
}

impl SingleExchangeMessage for TestAnnounce {
    fn exchange() -> &'static str { "TestAnnounce" }
}

impl TestAnnounce {
    pub async fn setup(ch: &Channel) -> Result<()> {
        ch.exchange_declare(
            "TestAnnounce",
            ExchangeKind::Fanout,
            ExchangeDeclareOptions { durable: true, ..Default::default() },
            Default::default()
        ).await?;

        Ok(())
    }
}
