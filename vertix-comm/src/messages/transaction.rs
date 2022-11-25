use aragog::DatabaseRecord;
use lapin::{Channel, ExchangeKind, options::{ExchangeDeclareOptions, QueueDeclareOptions}};
use serde::{Serialize, Deserialize};
use vertix_model::Note;

use crate::{error::Result, SingleExchangeMessage, RpcMessage};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum Action {
    PublishNote(Note),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub responses: Vec<ActionResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum ActionResponse {
    PublishNote(DatabaseRecord<Note>),
}

impl SingleExchangeMessage for Transaction {
    fn exchange() -> &'static str { "Transaction" }
}

impl RpcMessage for Transaction {
    type Response = TransactionResponse;
}

impl Transaction {
    pub async fn setup(ch: &Channel) -> Result<()> {
        // Transactions have an empty routing key.
        ch.exchange_declare(
            "Transaction",
            ExchangeKind::Direct,
            ExchangeDeclareOptions { durable: true, ..Default::default() },
            Default::default()
        ).await?;

        // The Transaction.process queue is for workers to listen to transaction requests and
        // execute them.
        ch.queue_declare(
            "Transaction.process",
            QueueDeclareOptions { durable: true, ..Default::default() },
            Default::default()
        ).await?;

        ch.queue_bind(
            "Transaction.process",
            "Transaction",
            "",
            Default::default(),
            Default::default()
        ).await?;

        Ok(())
    }
}
