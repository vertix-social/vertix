use lapin::Channel;
use anyhow::Result;
use aragog::Record;

use vertix_comm::ReceiveMessage;
use vertix_comm::messages::Interaction;
use vertix_model::{AragogConnectionManager, Account};

use futures::stream::StreamExt;

pub async fn listen(ch: &Channel, pool: bb8::Pool<AragogConnectionManager>) -> Result<()> {
    ch.basic_qos(2, Default::default()).await?;

    let mut stream = Interaction::receive(
        ch, "Interaction.for_remote", Default::default()).await?;

    log::debug!("listening for Interaction");

    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => {
                if let Err(err) = process_interaction(message.data(), &pool).await {
                    log::warn!("In process_transaction: error: {}", err);
                    message.nack().await?;
                } else {
                    message.ack().await?;
                }
            },
            Err(err) => {
                log::warn!("error in Interaction: {}", err);
            }
        }
    }

    Ok(())
}

async fn process_interaction(
    interaction: &Interaction,
    pool: &bb8::Pool<AragogConnectionManager>
) -> Result<()> {
    let db = pool.get().await?;

    match interaction {
        Interaction::Note(note) if note.from.is_some() => {
            let from = Account::find(note.from.as_ref().unwrap(), &*db).await?;
            if from.is_local() {
                log::warn!("TODO: Send note to inboxes of followers, recipients: {note:?}");
            }
        },
        Interaction::InitiateFollow {
            from_account, to_account, to_remote, ..
        } if *to_remote => {
            log::warn!("TODO: Send Follow to remote: {from_account} -> {to_account}");
        },
        Interaction::SetFollowAccepted {
            from_account, to_account, from_remote, accepted, ..
        } if *from_remote => {
            log::warn!("TODO: Send {}/Follow to remote {from_account} -> {to_account}",
                if *accepted { "Accept" } else { "Reject" });
        },
        _ => ()
    }
    Ok(())
}
