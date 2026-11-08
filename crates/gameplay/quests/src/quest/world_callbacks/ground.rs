//! Bounded mutable Ground evidence. This predicate grants no custody or mutation authority.
use serde::{Deserialize, Serialize};

pub const MAX_GROUND_STACKS: usize = 8;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GroundStackItem {
    pub item: String,
    pub revision: String,
    pub quantity: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GroundStackRequirement {
    /// Complete bottom-to-top sequence of live portable Ground instances on one tile.
    pub stacks: Vec<GroundStackItem>,
}

/// The custody owner supplies exact instance identities and its append ordinals.
/// The caller retains the current scope/map/content/tile locks while consuming this readset.
#[derive(Clone, Debug)]
pub struct GroundStackObserved<'a> {
    pub instance: [u8; 16],
    pub item: &'a str,
    pub revision: &'a str,
    pub quantity: u64,
    pub ordinal: Option<u64>,
}

impl GroundStackRequirement {
    pub fn valid(&self) -> bool {
        !self.stacks.is_empty()
            && self.stacks.len() <= MAX_GROUND_STACKS
            && self.stacks.iter().all(|item| {
                item.item.starts_with("oteryn:item.")
                    && super::super::valid_quest_key(&item.item)
                    && !item.revision.is_empty()
                    && item.revision.len() <= 128
                    && item
                        .revision
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
                    && (1..=100).contains(&item.quantity)
            })
    }

    pub fn matches(&self, observed: &[GroundStackObserved<'_>]) -> bool {
        self.valid()
            && self.stacks.len() == observed.len()
            && self
                .stacks
                .iter()
                .zip(observed)
                .enumerate()
                .all(|(index, (required, actual))| {
                    actual.instance != [0; 16]
                        && !observed[..index]
                            .iter()
                            .any(|before| before.instance == actual.instance)
                        && actual.ordinal.is_some_and(|ordinal| {
                            ordinal > 0
                                && (index == 0
                                    || observed[index - 1]
                                        .ordinal
                                        .is_some_and(|before| before < ordinal))
                        })
                        && actual.item == required.item
                        && actual.revision == required.revision
                        && actual.quantity == u64::from(required.quantity)
                })
    }
}
