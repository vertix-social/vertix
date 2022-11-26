use anyhow::Result;
use lapin::Channel;
use vertix_comm::messages::DeliverActivity;

use crate::process_queue;

pub async fn listen(ch: &Channel, client: reqwest::Client) -> Result<()> {
    log::debug!("Listening for DeliverActivity");

    process_queue(ch, "DeliverActivity.process",
        |data, _| process(data, &client)).await
}

async fn process(
    data: DeliverActivity,
    client: &reqwest::Client,
) -> Result<()> {
    log::debug!("Posting {data:?}");

    let body = serde_json::to_vec(&data.activity)?;

    client.post(data.inbox)
        .header(reqwest::header::CONTENT_TYPE, "application/activity+json")
        .body(body)
        .send()
        .await?;

    Ok(())
}
