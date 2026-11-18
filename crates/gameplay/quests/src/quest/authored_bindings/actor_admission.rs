//! Actor gates are explicit binding facts, checked against the chosen recipe at boot.
use super::{BoundQuest, Trigger};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActorAdmission {
    pub minimum_level: u32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub premium: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prerequisites: Vec<QuestPrerequisite>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personal_assignment: Option<PersonalAssignment>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PersonalAssignment {
    SpikeTasks,
}

impl PersonalAssignment {
    pub(super) const fn requirement(self) -> &'static str {
        match self {
            Self::SpikeTasks => {
                "Chosen level80 campaign admission; all three tier assignments available personally"
            }
        }
    }

    fn valid(self, quest: &BoundQuest) -> bool {
        match self {
            Self::SpikeTasks => {
                if quest.quest != "oteryn:quest.spike_task" {
                    return false;
                }
                let mut found_hunt = false;
                for stage in &quest.stages {
                    if let Trigger::Kill { counter } = &stage.trigger {
                        found_hunt = true;
                        // Acceptance is a gate on this character's assignment. The
                        // death writer checks the gate and increments in one receipt.
                        if stage.completes
                            || counter.encounter_source.is_some()
                            || !quest.tracks.iter().any(|track| {
                                track.key == counter.gate && track.initial != counter.gate_value
                            })
                        {
                            return false;
                        }
                    }
                }
                found_hunt
            }
        }
    }
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
    pub(super) fn validate(&self, quest: &BoundQuest) -> Result<(), String> {
        let mut labels = std::collections::BTreeSet::new();
        let mut quests = std::collections::BTreeSet::new();
        if self
            .personal_assignment
            .is_some_and(|assignment| !assignment.valid(quest))
            || self.prerequisites.len() > 16
            || self.prerequisites.iter().any(|required| {
                required.requirement.trim().is_empty()
                    || required.requirement == PersonalAssignment::SpikeTasks.requirement()
                    || required.requirement.len() > 512
                    || !crate::quest::valid_quest_key(&required.quest)
                    || !(required.quest.starts_with("oteryn:quest.")
                        || required.quest.starts_with("oteryn:quest/"))
                    || required.quest == quest.quest
                    || !labels.insert(&required.requirement)
                    || !quests.insert(&required.quest)
            })
        {
            return Err("invalid authored actor prerequisite".into());
        }
        Ok(())
    }
}
