//! Creature death can retire matching encounter adds in a bounded source area.
use super::Position;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::OnceLock};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeathCleanup {
    pub boss: String,
    pub add: String,
    pub minimum: Position,
    pub maximum: Position,
    pub sources: Vec<String>,
}

pub(super) fn validate(rows: &[DeathCleanup]) -> Result<(), String> {
    let creature_key = |key: &str| {
        key.strip_prefix("oteryn:creature.")
            .is_some_and(|suffix| !suffix.is_empty() && crate::quest::valid_quest_key(key))
    };
    let mut bosses = BTreeSet::new();
    if rows.len() > super::MAX_CALLBACKS
        || rows.iter().any(|row| {
            !creature_key(&row.boss)
                || !creature_key(&row.add)
                || row.boss == row.add
                || !bosses.insert(&row.boss)
                || row.minimum.x > row.maximum.x
                || row.minimum.y > row.maximum.y
                || row.minimum.floor != row.maximum.floor
                || !(-15..=0).contains(&row.minimum.floor)
                || row.sources.is_empty()
                || row.sources.len() > super::MAX_EFFECTS
                || row.sources.iter().any(|source| source.trim().is_empty())
        })
    {
        return Err("invalid encounter death cleanup binding".into());
    }
    Ok(())
}

fn read(text: &str) -> Result<Vec<DeathCleanup>, String> {
    let document: super::Document =
        serde_json::from_str(text).map_err(|error| error.to_string())?;
    if document.schema != 1 || document.callbacks.len() > super::MAX_CALLBACKS {
        return Err("callback bounds".into());
    }
    let mut cleanups = document.death_cleanups;
    validate(&cleanups)?;
    cleanups.sort_by(|left, right| left.boss.cmp(&right.boss));
    Ok(cleanups)
}

pub fn load() -> Result<&'static [DeathCleanup], String> {
    static BINDINGS: OnceLock<Result<Vec<DeathCleanup>, String>> = OnceLock::new();
    BINDINGS
        .get_or_init(|| {
            read(include_str!(
                "../../../../../../content/quests/authored-world-callbacks.json"
            ))
        })
        .as_deref()
        .map_err(Clone::clone)
}

#[cfg(any(test, feature = "test-support"))]
pub fn fixture(text: &str) -> Result<Vec<DeathCleanup>, String> {
    read(text)
}
