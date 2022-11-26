use lapin::Channel;
use serde::{Serialize, Deserialize};
use url::Url;
use activitystreams::activity::ActivityBox;
use crate::{SingleExchangeMessage, error::Result, macros::setup_exchange};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverActivity {
    pub inbox: Url,
    pub activity: ActivityBox,
}

impl SingleExchangeMessage for DeliverActivity {
    fn exchange() -> &'static str {
        "DeliverActivity"
    }
}

impl DeliverActivity {
    pub async fn setup(ch: &Channel) -> Result<()> {
        setup_exchange!(ch,
            DeliverActivity {
                kind Direct
                queues [
                    "DeliverActivity.process"
                ]
            }
        );

        Ok(())
    }
}