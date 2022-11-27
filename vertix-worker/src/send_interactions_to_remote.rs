use std::sync::Arc;

use activitystreams::activity;
use activitystreams::activity::properties::ActorAndObjectProperties;

use lapin::Channel;
use anyhow::Result;
use aragog::Record;
use anyhow::anyhow;

use vertix_comm::{SendMessage, ReceiveMessage};
use vertix_comm::messages::{Interaction, DeliverActivity};
use vertix_model::{AragogConnectionManager, Account};
use vertix_model::activitystreams::{UrlFor, ToObject};
use vertix_app_common::{Urls, Config};

use futures::stream::StreamExt;

pub async fn listen(
    ch: &Channel,
    config: Arc<Config>,
    pool: bb8::Pool<AragogConnectionManager>
) -> Result<()> {
    ch.basic_qos(2, Default::default()).await?;

    let mut stream = Interaction::receive(
        ch, "Interaction.for_remote", Default::default()).await?;

    log::debug!("listening for Interaction");

    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => {
                if let Err(err) = process_interaction(message.data(), ch, &*config, &pool).await {
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
    ch: &Channel,
    config: &Config,
    pool: &bb8::Pool<AragogConnectionManager>
) -> Result<()> {
    let db = pool.get().await?;

    let urls = Urls::new(&config.base_url, &*db);

    match interaction {
        Interaction::Note(note) if note.from.is_some() => {
            let from = Account::find(note.from.as_ref().unwrap(), &*db).await?;
            if from.is_local() {
                log::warn!("TODO: Send note to inboxes of followers, recipients: {note:?}");
            }
        },

        Interaction::InitiateFollow(follow) if follow.to_remote => {
            let to = urls.account_cache.get(follow.key_to(), &*db).await?;
            if to.is_remote() {
                log::debug!("Send Follow to remote {follow:?}");
                let follow_activity = follow.to_object::<_, anyhow::Error>(&urls).await?;
                let inbox = urls.url_for_account_inbox(&follow.key_to()).await?;
                DeliverActivity { inbox, activity: follow_activity.try_into()? }.send(&ch).await?;
            }
        },

        Interaction::SetFollowAccepted(follow) if follow.from_remote => {
            let accepted = follow.accepted.expect("Accepted not set even after SetFollowAccepted");
            let from = urls.account_cache.get(follow.key_from(), &*db).await?;
            if from.is_remote() {
                log::debug!("Send {}/Follow to remote {follow:?}",
                    if accepted { "Accept" } else { "Reject" });
                let follow_activity = follow.to_object::<_, anyhow::Error>(&urls).await?;
                let inbox = urls.url_for_account_inbox(&follow.key_to()).await?;
                let activity = if accepted {
                    make_follow_response::<activity::Accept>(follow_activity)?.try_into()?
                } else {
                    make_follow_response::<activity::Reject>(follow_activity)?.try_into()?
                };
                DeliverActivity { inbox, activity }.send(&ch).await?;
            }
        },
        _ => ()
    }
    Ok(())
}

fn make_follow_response<A>(follow: activity::Follow) -> Result<A>
    where A: activity::Activity + Default + AsMut<ActorAndObjectProperties>,
{
    let mut activity = A::default();
    activity.as_mut().set_actor_xsd_any_uri(
        follow.follow_props.get_object_xsd_any_uri()
            .ok_or_else(|| anyhow!("Object not set on follow {follow:?}"))?.clone())?;
    activity.as_mut().set_object_base_box(follow)?;
    Ok(activity)
}
