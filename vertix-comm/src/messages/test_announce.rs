use serde::{Serialize, Deserialize};

use lapin::Channel;

use crate::{error::Result, SingleExchangeMessage, macros::setup_exchange};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestAnnounce {
    pub message: String,
}

impl SingleExchangeMessage for TestAnnounce {
    fn exchange() -> &'static str { "TestAnnounce" }
}

impl TestAnnounce {
    pub async fn setup(ch: &Channel) -> Result<()> {
        setup_exchange!(ch, TestAnnounce { kind Fanout queues ["TestAnnounce"] });

        Ok(())
    }
}
