//! Actor gates are explicit binding facts, checked against the chosen recipe at boot.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActorAdmission {
    pub minimum_level: u32,
}
