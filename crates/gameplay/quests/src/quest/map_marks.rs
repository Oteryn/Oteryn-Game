//! Bounded personal quest markers; the Character receipt owns their durable creation.
use serde::{Deserialize, Serialize};

pub const MAX_PERSONAL_QUEST_MAP_MARKS: usize = 256;
pub const MAX_MAP_MARK_TEXT_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MapMarkKind {
    Cross,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QuestMapMark {
    pub position: super::world_callbacks::Position,
    pub kind: MapMarkKind,
    pub text: String,
}

impl QuestMapMark {
    #[must_use]
    pub fn valid(&self) -> bool {
        (-15..=0).contains(&self.position.floor)
            && !self.text.trim().is_empty()
            && self.text.len() <= MAX_MAP_MARK_TEXT_BYTES
            && self.text.chars().count() <= 128
            && !self.text.chars().any(char::is_control)
    }
}
