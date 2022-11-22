use anyhow::Result;
use aragog::Record;
use lapin::Channel;
use futures::stream::StreamExt;

use log::{warn, debug};
use vertix_comm::{SendMessage, ReceiveMessage, Delivery};
use vertix_comm::messages::{Transaction, Action, Interaction, TransactionResponse, ActionResponse};
use vertix_model::{AragogConnectionManager, Note, Account};

pub async fn listen(ch: &Channel, pool: bb8::Pool<AragogConnectionManager>) -> Result<()> {
    let mut stream = Transaction::receive(ch, "Transaction.process", Default::default()).await?;

    debug!("Listening for Transaction");

    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => {
                match execute(&message, ch, &pool).await {
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
    delivery: &Delivery<Transaction>,
    ch: &Channel,
    pool: &bb8::Pool<AragogConnectionManager>
) -> Result<()> {
    let transaction = delivery.data();

    debug!("Execute transaction: {:?}", transaction);

    let db = pool.get().await?;
    let db_trans = aragog::transaction::Transaction::new(&db).await?;

    let mut interactions = vec![];
    let mut responses = vec![];

    let result = (async {
        for action in &transaction.actions {
            let response = execute_action(action, &mut interactions, &*db).await?;
            responses.push(response);
        }
        Ok::<(), anyhow::Error>(())
    }).await;

    if result.is_ok() {
        db_trans.commit().await?;

        // After commit, we should just warn about any errors

        // Publish interactions
        for interaction in interactions {
            if let Err(err) = interaction.send(ch).await {
                warn!("Error publishing Interaction: {}", err);
            }
        }

        // Send reply
        if let Err(err) = delivery.reply(ch, &TransactionResponse { responses }).await {
            warn!("Error sending reply to Transaction: {}", err);
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
) -> Result<ActionResponse> {
    match action {
        Action::PublishNote { from, note } => {
            let account = Account::find(&from, &*db).await?;
            let note_doc = Note::publish(&account, note.clone(), &*db).await?;
            interactions.push(Interaction::Note {
                from: from.to_owned(),
                note: note_doc.clone()
            });
            Ok(ActionResponse::PublishNote(note_doc))
        },
    }
}
