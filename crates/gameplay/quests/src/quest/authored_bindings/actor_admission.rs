//! Actor gates are explicit binding facts, checked against the chosen recipe at boot.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActorAdmission {
    pub minimum_level: u32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub premium: bool,
}

fn is_false(value: &bool) -> bool {
    !value
}
