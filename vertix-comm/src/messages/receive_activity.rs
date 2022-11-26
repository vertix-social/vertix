use lapin::{Channel, ExchangeKind};
use lapin::options::{ExchangeDeclareOptions, QueueDeclareOptions};
use serde::{Serialize, Deserialize};
use activitystreams::activity::ActivityBox;
use crate::SingleExchangeMessage;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiveActivity {
    pub activity: ActivityBox,
}

impl SingleExchangeMessage for ReceiveActivity {
    fn exchange() -> &'static str {
        "ReceiveActivity"
    }
}

impl ReceiveActivity {
    pub async fn setup(ch: &Channel) -> Result<()> {
        ch.exchange_declare(
            "ReceiveActivity",
            ExchangeKind::Direct,
            ExchangeDeclareOptions { durable: true, ..Default::default() },
            Default::default()
        ).await?;

        ch.queue_declare(
            "ReceiveActivity.process",
            QueueDeclareOptions { durable: true, ..Default::default() },
            Default::default()
        ).await?;

        ch.queue_bind(
            "ReceiveActivity.process",
            "ReceiveActivity",
            "",
            Default::default(),
            Default::default()
        ).await?;

        Ok(())
    }
}
