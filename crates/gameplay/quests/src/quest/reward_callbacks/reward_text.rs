//! Literal initial text, either on a plain reward or on named children of its container.
use super::ItemAmount;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardContentText {
    /// Zero-based index in the authored, ordered `contents` plan.
    pub index: u16,
    pub text: String,
}

/// Existing string-valued rewards retain their exact serialized form and binding.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged, deny_unknown_fields)]
pub enum RewardText {
    Root(String),
    Contents { contents: Vec<RewardContentText> },
}

impl From<String> for RewardText {
    fn from(value: String) -> Self {
        Self::Root(value)
    }
}

impl From<&str> for RewardText {
    fn from(value: &str) -> Self {
        Self::Root(value.into())
    }
}

fn bounded(text: &str) -> bool {
    !text.is_empty() && text.len() <= 7_996 && text.chars().count() <= 4_000 && !text.contains('\0')
}

impl RewardText {
    pub fn root(&self) -> Option<&str> {
        match self {
            Self::Root(text) => Some(text),
            Self::Contents { .. } => None,
        }
    }

    pub fn content(&self, index: usize) -> Option<&str> {
        match self {
            Self::Root(_) => None,
            Self::Contents { contents } => contents
                .iter()
                .find(|entry| usize::from(entry.index) == index)
                .map(|entry| entry.text.as_str()),
        }
    }

    pub(super) fn valid_for(&self, count: u16, items: &[ItemAmount]) -> bool {
        if count != 1 {
            return false;
        }
        match self {
            Self::Root(text) => items.is_empty() && bounded(text),
            Self::Contents { contents } => {
                !contents.is_empty()
                    && contents.len() <= 20
                    && contents
                        .windows(2)
                        .all(|pair| pair[0].index < pair[1].index)
                    && contents.iter().all(|entry| {
                        bounded(&entry.text)
                            && items
                                .get(usize::from(entry.index))
                                .is_some_and(|item| item.count == 1 && item.fluid_subtype.is_none())
                    })
                    && contents.iter().map(|entry| entry.text.len()).sum::<usize>() <= 7_996
                    && contents
                        .iter()
                        .map(|entry| entry.text.chars().count())
                        .sum::<usize>()
                        <= 4_000
            }
        }
    }
}
