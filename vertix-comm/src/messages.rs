use lapin::{Channel};
use crate::error::Result;

mod test_announce;
mod transaction;
mod interaction;

pub use test_announce::*;
pub use transaction::*;
pub use interaction::*;

pub async fn setup(ch: &Channel) -> Result<()> {
    futures::try_join!(
        TestAnnounce::setup(ch),
        Transaction::setup(ch),
        Interaction::setup(ch),
    )?;

    Ok(())
}
