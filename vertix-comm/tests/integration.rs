use std::time::Duration;

use futures::Future;
use lapin::Channel;
use vertix_comm::*;
use vertix_comm::messages::*;
use test_log::test;
use anyhow::Result;
use futures::stream::StreamExt;
use log::debug;
use tokio::sync::oneshot;
use vertix_model::Recipient;
use serde_json::json;
use actix_rt::time::timeout;

async fn harness<
    A, B,
    SetupFn, SetupFut,
    SendFn, SendFut,
    RecvFn, RecvFut,
    PostFn, PostFut
>(
    setup: SetupFn,
    send: SendFn,
    recv: RecvFn,
    post: PostFn,
) -> Result<()>
where
    B: Send + 'static,
    SetupFn: FnOnce(Channel) -> SetupFut,
    SetupFut: Future<Output=Result<()>>,
    SendFn: FnOnce(Channel) -> SendFut,
    SendFut: Future<Output=Result<A>>,
    RecvFn: (FnOnce(Channel, oneshot::Sender<()>) -> RecvFut) + Send + 'static,
    RecvFut: Future<Output=Result<B>>,
    PostFn: FnOnce(Channel, A, B) -> PostFut,
    PostFut: Future<Output=Result<()>>,
{
    timeout(Duration::from_secs(1), async move {
        let connection = create_connection().await?;
        setup(connection.create_channel().await?).await?;

        let recv_channel = connection.create_channel().await?;

        let (ready_tx, ready_rx) = oneshot::channel();

        let task = actix_rt::spawn(async move { recv(recv_channel, ready_tx).await });

        // Wait for receiver to be ready
        ready_rx.await?;

        // Send the message
        let a = send(connection.create_channel().await?).await?;

        // Try to receive
        let b = task.await??;

        post(connection.create_channel().await?, a, b).await?;

        Ok(())
    }).await?
}

#[test(actix_rt::test)]
async fn send_and_receive_copy() -> Result<()> {
    let message = TestAnnounce { message: "Hello, world!".into() };

    harness(
        |ch| async move {
            TestAnnounce::setup(&ch).await?;
            debug!("Test announce setup done");
            Ok(())
        },
        |ch| async move {
            message.send(&ch).await?;
            Ok(message)
        },
        |ch, ready| async move {
            let mut stream = TestAnnounce::receive_copies(&ch, "").await?;

            debug!("Receive stream ready");
            let _ = ready.send(());

            stream.next().await.transpose().map_err(anyhow::Error::from)
        },
        |_ch, message, received_message| async move {
            debug!("Message received: {:?}", received_message);
            assert_eq!(Some(message), received_message);
            Ok(())
        }
    ).await
}

async fn interaction_listen_test(from: Vec<String>, to: Vec<Recipient>) -> Result<()> {
    let message: Interaction = serde_json::from_value(json!({
        "type": "Note",
        "params": {
            "from": "0001",
            "_id": "note/test-0001",
            "_key": "test-0001",
            "_rev": "_",
            "created_at": "2020-01-01T00:00:00Z",
            "to": ["public", {"account": "0002"}],
            "content": "Hello, world"
        }
    }))?;

    harness(
        |ch| async move { Ok(Interaction::setup(&ch).await?) },
        |ch| async move {
            message.send(&ch).await?;
            Ok(message)
        },
        |ch, ready| async move {
            let mut stream = Interaction::listen(&ch, &from, &to).await?;
            let _ = ready.send(());
            stream.next().await.transpose().map_err(anyhow::Error::from)
        },
        |_ch, message, received_message| async move {
            debug!("Message received: {:?}", received_message);
            assert_eq!(
                serde_json::to_string(&message)?,
                serde_json::to_string(&received_message)?
            );
            Ok(())
        }
    ).await
}

#[test(actix_rt::test)]
async fn interaction_listen_no_filter() -> Result<()> {
    interaction_listen_test(vec![], vec![]).await
}

#[test(actix_rt::test)]
async fn interaction_listen_filter_from() -> Result<()> {
    interaction_listen_test(vec!["0001".into()], vec![]).await
}

#[test(actix_rt::test)]
async fn interaction_listen_filter_to_public() -> Result<()> {
    interaction_listen_test(vec![], vec![Recipient::Public]).await
}

#[test(actix_rt::test)]
async fn interaction_listen_filter_to_acct() -> Result<()> {
    interaction_listen_test(vec![], vec![Recipient::Account("0002".into())]).await
}
