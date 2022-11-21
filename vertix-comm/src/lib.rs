use std::borrow::Cow;
use std::pin::Pin;

use lapin::options::{QueueDeclareOptions, BasicConsumeOptions};
use lapin::protocol::basic::AMQPProperties;
use lapin::{Channel, Connection};
use lapin::acker::Acker;
use lapin::publisher_confirm::PublisherConfirm;

use async_trait::async_trait;
use futures::stream::{Stream, TryStreamExt};
use serde::{Serialize, de::DeserializeOwned};

pub mod messages;
pub mod error;

pub use error::Error;
use error::Result;

#[async_trait(?Send)]
pub trait SendMessage {
    async fn send(&self, channel: &Channel) -> Result<PublisherConfirm>;
}

#[async_trait(?Send)]
pub trait ReceiveMessage: Sized {
    async fn receive(channel: &Channel, queue_name: &str)
        -> Result<Pin<Box<dyn Stream<Item=Result<Delivery<Self>>>>>>;

    async fn receive_copies(channel: &Channel, routing_key: &str)
        -> Result<Pin<Box<dyn Stream<Item=Result<Self>>>>>;
}

pub trait SingleExchangeMessage: Sized {
    fn exchange() -> &'static str;
    fn routing_key(&self) -> Cow<'_, str>;
}

#[async_trait(?Send)]
impl<M> SendMessage for M where M: SingleExchangeMessage + Serialize {
    async fn send(&self, channel: &Channel) -> Result<PublisherConfirm> {
        let serialized = serde_json::to_vec(&self)?;
        Ok(channel.basic_publish(
            Self::exchange(),
            &*self.routing_key(),
            Default::default(),
            &serialized[..],
            AMQPProperties::default()
        ).await?)
    }
}

#[async_trait(?Send)]
impl<M> ReceiveMessage for M where M: SingleExchangeMessage + DeserializeOwned + 'static {
    async fn receive(channel: &Channel, queue_name: &str)
        -> Result<Pin<Box<dyn Stream<Item=Result<Delivery<Self>>>>>> {
        let stream = channel.basic_consume(queue_name, "", BasicConsumeOptions {
            no_ack: true,
            ..Default::default()
        }, Default::default()).await?;

        Ok(Box::pin(stream.map_err(Error::from).and_then(|item| async move {
            let deserialized = serde_json::from_slice(&item.data[..])?;
            Ok(Delivery { data: deserialized, acker: item.acker })
        })))
    }

    async fn receive_copies(channel: &Channel, routing_key: &str)
        -> Result<Pin<Box<dyn Stream<Item=Result<Self>>>>>
    {
        let queue = channel.queue_declare("", QueueDeclareOptions {
            exclusive: true,
            ..Default::default()
        }, Default::default()).await?;

        channel.queue_bind(queue.name().as_str(), Self::exchange(), routing_key, Default::default(),
            Default::default()).await?;

        let stream = Self::receive(channel, queue.name().as_str()).await?;

        Ok(Box::pin(stream.map_ok(|item| item.data)))
    }
}

pub struct Delivery<M> {
    data: M,
    acker: Acker,
}

impl<M> Delivery<M> {
    pub fn data(&self) -> &M {
        &self.data
    }

    pub async fn ack(&self) -> Result<()> {
        Ok(self.acker.ack(Default::default()).await?)
    }

    pub async fn nack(&self) -> Result<()> {
        Ok(self.acker.nack(Default::default()).await?)
    }
}

pub async fn create_connection() -> Result<Connection> {
    let addr: String = std::env::var("AMQP_ADDR").map_err(|_| Error::AmqpConfigMissing)?;

    let conn = Connection::connect(&addr, Default::default()).await?;

    Ok(conn)
}

