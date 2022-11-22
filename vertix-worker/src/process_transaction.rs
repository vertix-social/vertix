use anyhow::Result;
use aragog::Record;
use lapin::Channel;
use futures::stream::StreamExt;

use log::{warn, debug};
use vertix_comm::{SendMessage, ReceiveMessage};
use vertix_comm::messages::{Transaction, Action, Interaction};
use vertix_model::{AragogConnectionManager, Note, Account};

pub async fn listen(ch: &Channel, pool: bb8::Pool<AragogConnectionManager>) -> Result<()> {
    let mut stream = Transaction::receive(ch, "Transaction.process", Default::default()).await?;

    debug!("Listening for Transaction");

    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => {
                match execute(message.data(), ch, &pool).await {
                    Ok(_) => {
                        message.ack().await?;
                    },
                    Err(err) => {
                        warn!("Failed to process Transaction: {err}");
                        message.nack().await?;
                    }
                }
            },
            Err(err) => {
                warn!("Can't decode Transaction: {err}");
            }
        }
    }

    Ok(())
}

async fn execute(
    transaction: &Transaction,
    ch: &Channel,
    pool: &bb8::Pool<AragogConnectionManager>
) -> Result<()> {
    debug!("Execute transaction: {:?}", transaction);

    let db = pool.get().await?;
    let db_trans = aragog::transaction::Transaction::new(&db).await?;

    let mut interactions = vec![];

    let result = (async {
        for action in &transaction.actions {
            execute_action(action, &mut interactions, &*db).await?;
        }
        Ok::<(), anyhow::Error>(())
    }).await;

    if result.is_ok() {
        db_trans.commit().await?;

        // Publish interactions
        for interaction in interactions {
            interaction.send(ch).await?;
        }
    } else {
        db_trans.abort().await?;
    }

    result
}

async fn execute_action(
    action: &Action,
    interactions: &mut Vec<Interaction>,
    db: &aragog::DatabaseConnection
) -> Result<()> {
    match action {
        Action::PublishNote { from, note } => {
            let account = Account::find(&from, &*db).await?;
            let note_doc = Note::publish(&account, note.clone(), &*db).await?;
            interactions.push(Interaction::Note {
                from: from.to_owned(),
                note: note_doc
            });
        },
    }
    Ok(())
}
