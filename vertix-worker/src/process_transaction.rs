use std::sync::Arc;

use activitystreams::{ext::Ext, actor::{Person, properties::ApActorProperties}};
use anyhow::{Result, anyhow, bail};
use aragog::Record;
use lapin::Channel;

use log::{warn, debug};
use vertix_comm::{SendMessage, Delivery};
use vertix_comm::messages::{Transaction, Action, Interaction, TransactionResponse, ActionResponse};
use vertix_model::{AragogConnectionManager, Note, Account, Follow};

use crate::process_queue;

pub async fn listen(
    ch: &Channel,
    pool: bb8::Pool<AragogConnectionManager>,
    client: reqwest::Client
) -> Result<()> {
    debug!("Listening for Transaction");

    process_queue(ch, "Transaction.process",
        |data, msg| execute(data, msg, ch, &pool, &client)).await
}

async fn execute(
    transaction: Transaction,
    msg: Arc<Delivery<()>>,
    ch: &Channel,
    pool: &bb8::Pool<AragogConnectionManager>,
    client: &reqwest::Client
) -> Result<()> {
    debug!("Execute transaction: {:?}", transaction);

    let db = pool.get().await?;
    let db_trans = aragog::transaction::Transaction::new(&db).await?;

    let mut interactions = vec![];
    let mut responses = vec![];

    let result = (async {
        for action in &transaction.actions {
            let response = execute_action(action, &mut interactions, &*db, client).await?;
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
        if let Err(err) = msg.reply(ch, &TransactionResponse { responses }).await {
            warn!("Error sending reply to Transaction: {}", err);
        }
    } else {
        db_trans.abort().await?;
    }

    result
}

static CONTENT_TYPES: &[&str] = &[
    "application/activity+json",
    "application/ld+json; profile = \"https://www.w3.org/ns/activitystreams\"",
];

async fn execute_action(
    action: &Action,
    interactions: &mut Vec<Interaction>,
    db: &aragog::DatabaseConnection,
    client: &reqwest::Client
) -> Result<ActionResponse> {
    match action {
        Action::FetchAccount(url) => {
            // Get the remote copy of the account
            let res = client.get(url.clone())
                .header(reqwest::header::ACCEPT, CONTENT_TYPES.join(", "))
                .send()
                .await?;

            let person: Ext<Person, ApActorProperties> = res.json().await?;
            let account_body: Account = person.try_into()?;
            let mut account;

            // Ensure the person's url matches the one we requested
            if account_body.remote.as_ref().unwrap().uri != *url {
                bail!("Fetched account URL does not match the one requested, url={}, fetched={:?}",
                    url, account_body);
            }

            // Try to get an existing account
            match Account::find_by_uri(url, db).await {
                Ok(existing_account) => {
                    // Update the existing account
                    account = existing_account;
                    account.remote = account_body.remote.clone();
                    account.updated_at = account_body.updated_at.clone();
                    account.save(db).await?;
                },
                Err(e) if e.is_not_found() => {
                    // Create a new account
                    account = Account::create(account_body, db).await?;
                },
                Err(e) => return Err(e.into()),
            }

            Ok(ActionResponse::FetchAccount(account))
        },

        Action::PublishNote(note) => {
            let from = note.from.as_ref()
                .ok_or_else(|| anyhow!("note.from must be set"))?;

            let account = Account::find(from, db).await?;

            let note_doc = Note::publish(&account, note.clone(), db).await?;
            interactions.push(Interaction::Note(note_doc.clone()));
            Ok(ActionResponse::PublishNote(note_doc))
        },

        Action::InitiateFollow { from_account, to_account } => {
            let actor = Account::find(&from_account, db).await?;
            let target = Account::find(&to_account, db).await?;

            let created;

            if Follow::find_between(&actor, &target, db).await.is_err() {
                Follow::link(&actor, &target, db).await?;

                interactions.push(Interaction::InitiateFollow {
                    from_account: actor.key().into(),
                    to_account: target.key().into(),
                    from_remote: actor.is_remote(),
                    to_remote: actor.is_remote(),
                });
                created = true;
            } else {
                created = false;
            }

            Ok(ActionResponse::InitiateFollow { created })
        },

        Action::SetFollowAccepted { from_account, to_account, accepted } => {
            let actor = Account::find(&from_account, db).await?;
            let target = Account::find(&to_account, db).await?;
            
            let mut follow = Follow::find_between(&actor, &target, db).await?;

            let modified;

            if follow.accepted != Some(*accepted) {
                follow.accepted = Some(*accepted);
                follow.save(db).await?;

                interactions.push(Interaction::SetFollowAccepted {
                    from_account: actor.key().into(),
                    to_account: target.key().into(),
                    from_remote: actor.is_remote(),
                    to_remote: actor.is_remote(),
                    accepted: *accepted,
                });
                modified = true;
            } else {
                modified = false;
            }

            Ok(ActionResponse::SetFollowAccepted { modified })
        }
    }
}
