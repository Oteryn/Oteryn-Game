//! Crystal 201189f8 data-global/scripts/quests/the_pits_of_inferno_quest/
//! globalevents_pits_of_inferno_bosses.lua:1-8,12-14,23.
use super::{Document, EncounterMonster};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeriodicBoss {
    pub key: String,
    pub interval_ms: u32,
    pub cooldown_ms: u32,
    #[serde(default)]
    pub placement: PeriodicBossPlacement,
    pub choices: Vec<WeightedBoss>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PeriodicBossPlacement {
    #[default]
    Exact,
    ExtendedForced,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeightedBoss {
    pub weight: u16,
    pub spawn: EncounterMonster,
}

pub(super) fn valid(plans: &[PeriodicBoss]) -> bool {
    let mut keys = BTreeSet::new();
    plans.len() <= 64
        && plans.iter().all(|plan| {
            crate::quest::valid_quest_key(&plan.key)
                && keys.insert(&plan.key)
                && plan.interval_ms > 0
                && (1..=64).contains(&plan.choices.len())
                && plan.choices.iter().all(|choice| {
                    choice.weight > 0
                        && (-15..=0).contains(&choice.spawn.position.floor)
                        && crate::quest::valid_quest_key(&choice.spawn.creature)
                })
        })
}

pub fn load() -> Result<&'static [PeriodicBoss], &'static str> {
    static PLANS: OnceLock<Result<Vec<PeriodicBoss>, &'static str>> = OnceLock::new();
    PLANS
        .get_or_init(|| {
            let document: Document = serde_json::from_str(include_str!(
                "../../../../../../content/quests/authored-world-callbacks.json"
            ))
            .map_err(|_| "invalid periodic boss source document")?;
            if document.schema != 1 || !valid(&document.periodic_bosses) {
                return Err("invalid periodic boss bindings");
            }
            Ok(document.periodic_bosses)
        })
        .as_deref()
        .map_err(|error| *error)
}
