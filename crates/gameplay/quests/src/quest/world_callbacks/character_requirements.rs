//! M03: chosen 25 Years recipe requires promotion and regular blessings.
//! Premium: content/quests/definitions/quests-00000-00099.json:1, record 89
//! (Pits of Inferno chosen recipe requirements).
//! Evidence: content/quests/definitions/quests-00200-00299.json:1 (record 67,
//! requirements); donor actions_time_machine.lua:26-35 supplies the world trigger,
//! not these owner-selected prerequisites.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CharacterRequirements {
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub premium: bool,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub promoted: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blessings: Vec<String>,
}

impl CharacterRequirements {
    pub(super) fn valid(&self) -> bool {
        (self.premium || self.promoted || !self.blessings.is_empty())
            && self.blessings.len() <= 8
            && self.blessings.iter().all(|key| {
                key.len() <= 128
                    && key.strip_prefix("oteryn:blessing.").is_some_and(|stem| {
                        !stem.is_empty()
                            && stem
                                .bytes()
                                .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
                    })
            })
            && self.blessings.iter().collect::<BTreeSet<_>>().len() == self.blessings.len()
    }
}
