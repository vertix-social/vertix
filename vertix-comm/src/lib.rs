use std::borrow::Cow;
use std::pin::Pin;

use futures::try_join;
use lapin::options::{QueueDeclareOptions, BasicConsumeOptions};
use lapin::protocol::basic::AMQPProperties;
use lapin::{Channel, Connection};
use lapin::acker::Acker;
use lapin::publisher_confirm::PublisherConfirm;

use async_trait::async_trait;
use futures::stream::{Stream, StreamExt, TryStreamExt};
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
    async fn receive(channel: &Channel, queue_name: &str, options: BasicConsumeOptions)
        -> Result<Pin<Box<dyn Stream<Item=Result<Delivery<Self>>>>>>;

    async fn receive_copies(channel: &Channel, routing_key: &str)
        -> Result<Pin<Box<dyn Stream<Item=Result<Self>>>>>;
}

pub trait SingleExchangeMessage: Sized {
    fn exchange() -> &'static str;

    fn routing_key(&self) -> Cow<'_, str> {
        "".into()
    }

    fn amqp_properties(&self) -> AMQPProperties {
        AMQPProperties::default()
    }
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
            self.amqp_properties()
        ).await?)
    }
}

#[async_trait(?Send)]
impl<M> ReceiveMessage for M where M: SingleExchangeMessage + DeserializeOwned + 'static {
    async fn receive(channel: &Channel, queue_name: &str, options: BasicConsumeOptions)
        -> Result<Pin<Box<dyn Stream<Item=Result<Delivery<Self>>>>>> {
        let stream = channel.basic_consume(queue_name, "", options, Default::default()).await?;

        Ok(Box::pin(stream.map_err(Error::from).and_then(|item| async move {
            let deserialized = serde_json::from_slice(&item.data[..])?;
            Ok(Delivery {
                data: deserialized,
                acker: item.acker,
                properties: item.properties
            })
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

        let stream = Self::receive(channel, queue.name().as_str(), BasicConsumeOptions {
            no_ack: true, // make sure acker is unnecessary
            ..Default::default()
        }).await?;

        Ok(Box::pin(stream.map_ok(|item| item.data)))
    }
}

pub struct Delivery<M> {
    data: M,
    acker: Acker,
    properties: AMQPProperties
}

impl<M> Delivery<M> {
    pub fn data(&self) -> &M {
        &self.data
    }

    pub fn properties(&self) -> &AMQPProperties {
        &self.properties
    }

    pub async fn ack(&self) -> Result<()> {
        Ok(self.acker.ack(Default::default()).await?)
    }

    pub async fn nack(&self) -> Result<()> {
        Ok(self.acker.nack(Default::default()).await?)
    }

    pub fn wants_reply(&self) -> bool {
        self.properties.reply_to().is_some()
    }

    pub async fn reply(&self, channel: &Channel, body: &impl Serialize) -> Result<()> {
        if let Some(reply_to) = self.properties.reply_to() {
            let serialized = serde_json::to_vec(body)?;

            channel.basic_publish(
                "",
                reply_to.as_str(),
                Default::default(),
                &serialized,
                Default::default()
            ).await?;
        }

        Ok(())
    }
}

#[async_trait(?Send)]
pub trait RpcMessage: SingleExchangeMessage + Serialize {
    type Response: DeserializeOwned;

    async fn remote_call(&self, channel: &Channel) -> Result<Self::Response> {
        let serialized = serde_json::to_vec(&self)?;

        let routing_key = &*self.routing_key();

        let mut stream = channel.basic_consume(
            "amq.rabbitmq.reply-to",
            "",
            BasicConsumeOptions { no_ack: true, ..Default::default() },
            Default::default()
        ).await?;

        let (received_reply, _) = try_join!(
            async { stream.next().await.transpose() },
            channel.basic_publish(
                Self::exchange(),
                routing_key,
                Default::default(),
                &serialized[..],
                self.amqp_properties().with_reply_to("amq.rabbitmq.reply-to".into())
            )
        )?;

        let deserialized = serde_json::from_slice(
            &received_reply.ok_or(Error::NoReply)?.data)?;

        Ok(deserialized)
    }
}

pub async fn create_connection() -> Result<Connection> {
    let addr: String = std::env::var("AMQP_ADDR").map_err(|_| Error::AmqpConfigMissing)?;

    let conn = Connection::connect(&addr, Default::default()).await?;

    Ok(conn)
}

