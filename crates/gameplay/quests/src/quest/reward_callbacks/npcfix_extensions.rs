//! NPCFIX lifetime, workshop, repeat and bulk effects over the main reward contract.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DecayTarget {
    pub family: String,
    pub key: String,
    pub revision: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardDecay {
    pub duration_millis: u32,
    pub target: Option<DecayTarget>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BulkExchange {
    pub item: String,
    pub output: String,
    pub base: u16,
    pub bonus: u16,
    pub chance_percent: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardClock {
    pub track: String,
    pub elapsed_seconds: Option<i64>,
    pub set_now: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardCooldown {
    pub track: String,
    pub seconds: u32,
}
