//! Source-qualified encounter cleanup and fishing backlash bindings.
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FishingFailure {
    pub water_effect: u16,
    pub shock_effect: u16,
    pub shock_chance_percent: u8,
    pub damage: u32,
    pub look_type: u16,
    pub duration_ms: u32,
    pub speech: String,
    pub sources: Vec<String>,
}

#[derive(Debug)]
pub struct SourceBindings {
    pub death_cleanups: Vec<DeathCleanup>,
    pub fishing_failure: Option<FishingFailure>,
}

pub(super) fn validate(document: &super::Document) -> Result<(), String> {
    let rows = &document.death_cleanups;
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
                || !sources_valid(&row.sources)
        })
    {
        return Err("invalid encounter death cleanup binding".into());
    }
    if document.fishing_failure.as_ref().is_some_and(|row| {
        row.water_effect == 0
            || row.shock_effect == 0
            || !(1..=100).contains(&row.shock_chance_percent)
            || row.damage == 0
            || row.look_type == 0
            || !(1..=3_600_000).contains(&row.duration_ms)
            || row.speech.trim().is_empty()
            || row.speech.len() > 255
            || !sources_valid(&row.sources)
    }) {
        return Err("invalid fishing failure binding".into());
    }
    Ok(())
}

fn sources_valid(sources: &[String]) -> bool {
    !sources.is_empty()
        && sources.len() <= super::MAX_EFFECTS
        && sources.iter().all(|source| !source.trim().is_empty())
}

fn read(text: &str) -> Result<SourceBindings, String> {
    let document: super::Document =
        serde_json::from_str(text).map_err(|error| error.to_string())?;
    if document.schema != 1 || document.callbacks.len() > super::MAX_CALLBACKS {
        return Err("callback bounds".into());
    }
    validate(&document)?;
    let mut cleanups = document.death_cleanups;
    cleanups.sort_by(|left, right| left.boss.cmp(&right.boss));
    Ok(SourceBindings {
        death_cleanups: cleanups,
        fishing_failure: document.fishing_failure,
    })
}

pub fn load() -> Result<&'static SourceBindings, String> {
    static BINDINGS: OnceLock<Result<SourceBindings, String>> = OnceLock::new();
    BINDINGS
        .get_or_init(|| {
            read(include_str!(
                "../../../../../../content/quests/authored-world-callbacks.json"
            ))
        })
        .as_ref()
        .map_err(Clone::clone)
}

#[cfg(any(test, feature = "test-support"))]
pub fn fixture(text: &str) -> Result<SourceBindings, String> {
    read(text)
}
