use vertix_comm::*;
use vertix_comm::messages::TestAnnounce;
use test_log::test;
use anyhow::Result;
use futures::stream::StreamExt;
use log::debug;
use tokio::sync::oneshot;

#[test(actix_rt::test)]
async fn send_and_receive_copy() -> Result<()> {
    let connection = create_connection().await?;

    let channel = connection.create_channel().await?;

    messages::setup(&channel).await?;
    
    debug!("Messages setup done");

    let alt_channel = connection.create_channel().await?;

    let (ready_tx, ready_rx) = oneshot::channel();

    let task = actix_rt::spawn(async move {
        let mut stream = TestAnnounce::receive_copies(&alt_channel, "").await?;

        debug!("Receive stream ready");
        let _ = ready_tx.send(());

        stream.next().await.transpose().map_err(anyhow::Error::from)
    });

    // Wait for receiver to be ready
    ready_rx.await?;

    // Send the message
    let message = TestAnnounce { message: "Hello, world!".into() };
    message.send(&channel).await?;

    debug!("Message sent: {:?}", message);

    // Try to receive
    let received_message = task.await??;

    debug!("Message received: {:?}", received_message);

    assert_eq!(Some(message), received_message);

    Ok(())
}
