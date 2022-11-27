use lapin::Channel;
use lapin::types::{FieldTable, LongString};
use lapin::options::{QueueDeclareOptions, BasicConsumeOptions};
use lapin::protocol::basic::AMQPProperties;

use serde::{Serialize, Deserialize};
use futures::stream::{Stream, TryStreamExt};
use vertix_model::{Note, Recipient, Document, Edge, Follow};

use crate::{SingleExchangeMessage, ReceiveMessage, macros::setup_exchange, error::Result};

/// Announces that an interaction has been committed.
///
/// Exchange type: headers.
///
/// # Headers
///
/// | key                    | value                                                      |
/// |------------------------|------------------------------------------------------------|
/// | `v-from-acct-{key}`    | true if account {key} is the sender/initiator              |
/// | `v-to-public`          | true if public is included in recipients                   |
/// | `v-to-acct-{key}`      | true if account {key} is included in recipients            |
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum Interaction {
    Note(Document<Note>),
    InitiateFollow(Edge<Follow>),
    SetFollowAccepted(Edge<Follow>),
}

impl SingleExchangeMessage for Interaction {
    fn exchange() -> &'static str {
        "Interaction"
    }

    fn amqp_properties(&self) -> AMQPProperties {
        let mut headers = FieldTable::default();

        // v-from
        match self {
            Interaction::Note(note) =>
                if let Some(ref from) = note.from {
                    headers.insert(format!("v-from-acct-{from}").into(), true.into())
                },
            Interaction::InitiateFollow(follow) |
            Interaction::SetFollowAccepted(follow) =>
                headers.insert(format!("v-from-acct-{}", follow.key_from()).into(), true.into()),
        }

        // v-to-*
        match self {
            Interaction::Note(note) => {
                for list in [&note.to, &note.cc, &note.bto, &note.bcc] {
                    for recipient in list {
                        match recipient {
                            Recipient::Public =>
                                headers.insert("v-to-public".into(), true.into()),
                            Recipient::Account(key) =>
                                headers.insert(format!("v-to-acct-{key}").into(), true.into()),
                        }
                    }
                }
            },
            Interaction::InitiateFollow(follow) |
            Interaction::SetFollowAccepted(follow) =>
                headers.insert(format!("v-to-acct-{}", follow.key_to()).into(), true.into()),
        }

        AMQPProperties::default().with_headers(headers)
    }
}

impl Interaction {
    pub async fn setup(ch: &Channel) -> Result<()> {
        setup_exchange!(ch,
            Interaction {
                kind Headers
                queues [
                    // For sending interactions to remote federated servers
                    "Interaction.for_remote",
                ]
            }
        );

        Ok(())
    }

    /// Listen to copies of interactions with the specified filters.
    ///
    /// Leave the filters as empty arrays if no filtering is desired.
    pub async fn listen(ch: &Channel, from: &[impl AsRef<str>], to: &[Recipient]) 
        -> Result<impl Stream<Item=Result<Interaction>>>
    {
        let queue = ch.queue_declare(
            "",
            QueueDeclareOptions { exclusive: true, ..Default::default() },
            Default::default()
        ).await?;

        let mut headers = FieldTable::default();

        for from_key in from {
            headers.insert(format!("v-from-acct-{}", from_key.as_ref()).into(), true.into());
        }

        for to_key in to {
            match to_key {
                Recipient::Public =>
                    headers.insert("v-to-public".into(), true.into()),
                Recipient::Account(key) =>
                    headers.insert(format!("v-to-acct-{key}").into(), true.into())
            }
        }

        if !headers.inner().is_empty() {
            headers.insert("x-match".into(), LongString::from("any").into());
        }

        log::debug!("Listening to headers {:?}", headers);

        ch.queue_bind(
            queue.name().as_str(),
            "Interaction",
            "",
            Default::default(),
            headers
        ).await?;

        Ok(Self::receive(ch, queue.name().as_str(),
            BasicConsumeOptions { no_ack: true, ..Default::default() }).await?
            .map_ok(|item| item.data))
    }
}
