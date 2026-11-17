//! Actor gates are explicit binding facts, checked against the chosen recipe at boot.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActorAdmission {
    pub minimum_level: u32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub premium: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prerequisites: Vec<QuestPrerequisite>,
}

fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QuestPrerequisite {
    pub requirement: String,
    pub quest: String,
}

impl ActorAdmission {
    pub(super) fn validate(&self, own_quest: &str) -> Result<(), String> {
        let mut labels = std::collections::BTreeSet::new();
        let mut quests = std::collections::BTreeSet::new();
        if self.prerequisites.len() > 16
            || self.prerequisites.iter().any(|required| {
                required.requirement.trim().is_empty()
                    || required.requirement.len() > 512
                    || !crate::quest::valid_quest_key(&required.quest)
                    || !(required.quest.starts_with("oteryn:quest.")
                        || required.quest.starts_with("oteryn:quest/"))
                    || required.quest == own_quest
                    || !labels.insert(&required.requirement)
                    || !quests.insert(&required.quest)
            })
        {
            return Err("invalid authored actor prerequisite".into());
        }
        Ok(())
    }
}
