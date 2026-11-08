//! Exact carried quest tools: no NPC speech can stand in for a selected target.
use super::{ItemAmount, RewardCallback, StateCondition, StateMutation};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum QuestToolTarget {
    Creature {
        creature: String,
        capture: bool,
    },
    Corpse {
        creature: String,
        appearance: u32,
        maximum_age_ms: u32,
    },
}

impl QuestToolTarget {
    pub fn creature(&self) -> &str {
        match self {
            Self::Creature { creature, .. } | Self::Corpse { creature, .. } => creature,
        }
    }
    pub fn captures(&self) -> bool {
        matches!(self, Self::Creature { capture: true, .. })
    }
    fn valid(&self) -> bool {
        let key = self.creature();
        key.len() <= 128
            && key.strip_prefix("oteryn:creature.").is_some_and(|tail| {
                !tail.is_empty()
                    && tail
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b))
            })
            && match self {
                Self::Creature { .. } => true,
                Self::Corpse {
                    appearance,
                    maximum_age_ms,
                    ..
                } => *appearance > 0 && (1..=3_600_000).contains(maximum_age_ms),
            }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QuestToolAction {
    pub target: QuestToolTarget,
    pub reusable: bool,
    pub replacement: Option<String>,
}

impl QuestToolAction {
    pub(super) fn valid(&self, callback: &RewardCallback) -> bool {
        let Some(item) = &callback.carried_item else {
            return false;
        };
        let exact = [ItemAmount {
            item: item.clone(),
            count: 1,
            fluid_subtype: None,
        }];
        self.target.valid()
            && callback.peer_requires.is_none()
            && callback.give.is_none()
            && callback.fee.is_none()
            && callback.cosmetic_targets().next().is_none()
            && callback.participants.is_empty()
            && callback.achievement.is_none()
            && callback.achievement_progress.is_empty()
            && if self.reusable {
                self.replacement.is_none() && callback.take.is_empty() && callback.present == exact
            } else {
                callback.take == exact
                    && callback.present.is_empty()
                    && self.replacement.as_deref().is_none_or(|key| {
                        super::super::valid_quest_key(key) && key.starts_with("oteryn:item.")
                    })
            }
    }
    pub(super) fn bounded_counter(callback: &RewardCallback) -> bool {
        callback.state_ops.iter().any(|op| matches!((&op.from, &op.effect),
            (StateCondition::Between { minimum, maximum }, StateMutation::Add { value })
            if *minimum >= 0 && minimum <= maximum && *value > 0 && maximum.checked_add(*value).is_some()))
    }
}
