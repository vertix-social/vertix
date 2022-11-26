use aragog::DatabaseRecord;
use lapin::Channel;
use serde::{Serialize, Deserialize};
use url::Url;
use vertix_model::{Note, Account};

use crate::{error::Result, SingleExchangeMessage, RpcMessage, macros::setup_exchange};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum Action {
    /// Get or update a remote account.
    FetchAccount(Url),
    /// Publish a note.
    PublishNote(Note),
    /// Start a follow between two accounts.
    InitiateFollow {
        from_account: String,
        to_account: String,
    },
    /// Accept or reject a follow between two accounts.
    SetFollowAccepted {
        from_account: String,
        to_account: String,
        accepted: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub responses: Vec<ActionResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum ActionResponse {
    FetchAccount(DatabaseRecord<Account>),
    PublishNote(DatabaseRecord<Note>),
    InitiateFollow { created: bool },
    SetFollowAccepted { modified: bool },
}

impl SingleExchangeMessage for Transaction {
    fn exchange() -> &'static str { "Transaction" }
}

impl RpcMessage for Transaction {
    type Response = TransactionResponse;
}

impl Transaction {
    pub async fn setup(ch: &Channel) -> Result<()> {
        setup_exchange!(ch,
            Transaction {
                // Transactions have an empty routing key.
                kind Direct
                queues [
                    // The Transaction.process queue is for workers to listen to transaction
                    // requests and execute them.
                    "Transaction.process",
                ]
            }
        );

        Ok(())
    }
}
