use std::sync::Arc;

use activitystreams::{ext::Ext, actor::{Person, properties::ApActorProperties}};
use anyhow::{Result, anyhow, bail};
use aragog::{Record, EdgeRecord};
use chrono::Utc;
use lapin::Channel;

use log::{warn, debug};
use vertix_comm::{SendMessage, Delivery};
use vertix_comm::messages::{Transaction, Action, Interaction, TransactionResponse, ActionResponse};
use vertix_model::{AragogConnectionManager, Note, Account, Follow, Wrap, Edge};

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
    "application/ld+json",
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
                .await?
                .error_for_status()?;

            let person: Ext<Person, ApActorProperties> = res.json().await?;
            let mut account_body: Account = person.try_into()?;
            let mut account;

            // We just fetched it, so set the right fetch time
            account_body.remote.as_mut().unwrap().last_fetched_at = Some(Utc::now());

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
                    account = Account::create(account_body, db).await?.wrap();
                },
                Err(e) => return Err(e.into()),
            }

            Ok(ActionResponse::FetchAccount(account))
        },

        Action::PublishNote(note) => {
            let from = note.from.as_ref()
                .ok_or_else(|| anyhow!("note.from must be set"))?;

            let account = Account::find(from, db).await?.wrap();

            let note_doc = Note::publish(&account, note.clone(), db).await?;
            interactions.push(Interaction::Note(note_doc.clone()));
            Ok(ActionResponse::PublishNote(note_doc))
        },

        Action::InitiateFollow { from_account, to_account, uri } => {
            let actor = Account::find(&from_account, db).await?;
            let target = Account::find(&to_account, db).await?;

            let created;
            let follow;

            if let Ok(found_follow) = Follow::find_between(&actor, &target, db).await {
                follow = found_follow;
                created = false;
            } else {
                follow = Follow::link(&actor, &target, uri.clone(), db).await?;
                created = true;

                interactions.push(Interaction::InitiateFollow(follow.clone()));
            }

            Ok(ActionResponse::InitiateFollow { created, follow })
        },

        Action::SetFollowAccepted { key, accepted } => {
            let mut follow: Edge<Follow> = EdgeRecord::<Follow>::find(&key, db).await?.wrap();

            let modified;

            if follow.accepted != Some(*accepted) {
                follow.accepted = Some(*accepted);
                follow.save(db).await?;

                interactions.push(Interaction::SetFollowAccepted(follow.clone()));
                modified = true;
            } else {
                modified = false;
            }

            Ok(ActionResponse::SetFollowAccepted { modified, follow })
        }
    }
}
