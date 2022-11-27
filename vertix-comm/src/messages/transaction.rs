use lapin::{Channel, publisher_confirm::PublisherConfirm};
use serde::{Serialize, Deserialize};
use url::Url;
use vertix_model::{Note, Account, Follow, Edge, Document};

use crate::{
    error::{Error, Result},
    macros::setup_exchange,
    SingleExchangeMessage,
    SendMessage,
    RpcMessage,
};

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
        uri: Option<Url>,
    },
    /// Accept or reject a follow between two accounts.
    SetFollowAccepted {
        key: String,
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
    FetchAccount(Document<Account>),
    PublishNote(Document<Note>),
    InitiateFollow { created: bool, follow: Edge<Follow> },
    SetFollowAccepted { modified: bool, follow: Edge<Follow> },
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

impl Action {
    pub fn into_transaction(self) -> Transaction {
        Transaction { actions: vec![self] }        
    }

    pub async fn send(self, ch: &Channel) -> Result<PublisherConfirm> {
        self.into_transaction().send(ch).await
    }

    pub async fn remote_call(self, ch: &Channel) -> Result<ActionResponse> {
        let TransactionResponse { mut responses } =
            self.into_transaction().remote_call(ch).await?;

        if responses.len() == 1 {
            Ok(responses.pop().unwrap())
        } else {
            Err(Error::InvalidReply(
                format!("should have exactly one response: {:?}", responses).into()))
        }
    }
}
