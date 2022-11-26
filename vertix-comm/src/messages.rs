use lapin::{Channel};
use crate::error::Result;

mod test_announce;
mod transaction;
mod interaction;
mod receive_activity;

pub use test_announce::*;
pub use transaction::*;
pub use interaction::*;
pub use receive_activity::*;

pub async fn setup(ch: &Channel) -> Result<()> {
    futures::try_join!(
        TestAnnounce::setup(ch),
        Transaction::setup(ch),
        Interaction::setup(ch),
        ReceiveActivity::setup(ch),
    )?;

    Ok(())
}
