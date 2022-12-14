use lapin::Channel;
use serde::{Serialize, Deserialize};
use activitystreams::activity::ActivityBox;
use crate::{SingleExchangeMessage, error::Result, macros::setup_exchange};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiveActivity {
    pub activity: ActivityBox,
}

impl SingleExchangeMessage for ReceiveActivity {
    fn exchange() -> &'static str {
        "ReceiveActivity"
    }
}

impl ReceiveActivity {
    pub async fn setup(ch: &Channel) -> Result<()> {
        setup_exchange!(ch,
            ReceiveActivity {
                kind Direct
                queues [
                    "ReceiveActivity.process"
                ]
            }
        );

        Ok(())
    }
}
