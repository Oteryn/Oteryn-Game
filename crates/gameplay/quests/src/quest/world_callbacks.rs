//! QUESTCB2: bounded, declarative world callbacks. Coordinates use native negative floors.
pub use super::native_rules::NativeRule;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const EMBEDDED: &str = include_str!("../../../../../content/quests/world-callbacks.json");
// Static callbacks and the dynamic altar share the same boot bound.
pub const MAX_CALLBACKS: usize = 512;
pub const MAX_EFFECTS: usize = 8;
mod objects;
pub use objects::{ObjectEffects, ObjectMutation};

// Exact corpus-backed sources additionally verified by source_world_objects.py.
// This admits only these files, not arbitrary callbacks at a different revision.
const SOURCE_WORLD_REFS: &[&str] = &[
    "crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/quests/hidden_threats/movement-chests_check.lua",
    "crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/quests/mintwallin_quest/actions_devil_helmet_lever.lua",
    "crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/quests/thais_quest/actions_life_ring_lever.lua",
];

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub x: u16,
    pub y: u16,
    pub floor: i8,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    Use,
    StepIn,
    StepOut,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    pub position: Option<Position>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub positions: Vec<Position>,
    pub unique: Option<u16>,
    pub action: Option<u16>,
    pub appearance: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Passage {
    pub position: Position,
    pub appearance: u16,
    #[serde(default)]
    pub replacement: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Offering {
    pub position: Position,
    pub item: String,
    pub count: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Storage {
    pub track: String,
    pub minimum: i64,
    pub maximum: i64,
    pub set: i64,
    /// Replace only this exact locked value; other values are retained.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_if: Option<i64>,
    /// Preserve the track value within a receipt-bearing quest transition.
    #[serde(default, skip_serializing_if = "is_false")]
    pub preserve: bool,
    /// Validate quest progress without writing tracks, state, receipts or revisions.
    #[serde(default, skip_serializing_if = "is_false")]
    pub read_only: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

mod native;
use native::valid_native_callback;

/// One carried item applied to the selected world object.  This deliberately is not a
/// recipe language: the source is consumed or changed exactly once by the receipt owner.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemOnTarget {
    pub item: String,
    pub count: u16,
    pub replacement: Option<String>,
    pub target_replacement: Option<u16>,
}

/// A source-world operation relocates every occupant before changing its cells.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OccupantEjection {
    pub positions: Vec<Position>,
    pub destination: Position,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MonkReward {
    pub item_source_id: Option<u32>,
    pub learn_spell: Option<String>,
    #[serde(default)]
    pub final_shrine: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Callback {
    pub key: String,
    pub quest: String,
    pub trigger: Trigger,
    pub selector: Selector,
    /// Both lever appearances. None for a step tile. One-way levers refuse their second use.
    pub lever: Option<[u16; 2]>,
    pub reversible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eject_occupants: Option<OccupantEjection>,
    pub passages: Vec<Passage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objects: Option<ObjectEffects>,
    pub teleport: Option<Position>,
    pub storage: Vec<Storage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_claims: Vec<String>,
    /// Checked against the progression owner in the same fenced quest transaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monk_reward: Option<MonkReward>,
    pub offerings: Vec<Offering>,
    #[serde(default)]
    pub item_on_target: Option<ItemOnTarget>,
    pub return_on_failure: bool,
    /// Explicit recovery landing for a refused STEP; absent preserves its origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_teleport: Option<Position>,
    pub failure_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_rule: Option<NativeRule>,
    pub sources: Vec<String>,
}

impl Callback {
    /// Only exact retained source SETs with fenced missing-claim predicates may
    /// write on departure; ordinary stateless recovery retains its stricter rule.
    #[must_use]
    pub fn is_guarded_source_departure(&self) -> bool {
        self.trigger == Trigger::StepOut
            && !self.required_claims.is_empty()
            && !self.storage.is_empty()
            && self.storage.iter().all(|storage| {
                !storage.preserve
                    && !storage.read_only
                    && storage.set_if.is_none()
                    && super::loader::inexact_source_set(&self.key, &storage.track)
                        == Some(storage.set)
            })
            && self.lever.is_none()
            && self.objects.is_none()
            && self.eject_occupants.is_none()
            && self.passages.is_empty()
            && self.teleport.is_none()
            && self.offerings.is_empty()
            && self.item_on_target.is_none()
            && self.minimum_level.is_none()
            && self.experience.is_none()
            && self.monk_reward.is_none()
            && self.native_rule.is_none()
            && !self.return_on_failure
    }

    /// Resolve only a retained inexact source SET whose missing predicates are
    /// supplied by this callback's fenced claim checks. The catalogue and its
    /// historical definition hash remain unchanged; generic evaluation stays closed.
    #[must_use]
    pub fn bind_inexact_source_transition(
        &self,
        source: &QuestTransition,
    ) -> Option<QuestTransition> {
        if self.required_claims.is_empty()
            || source.key != self.key
            || source.quest != self.quest
            || source.effects.len() != self.storage.len()
            || source.completes
            || source.experience != self.experience
        {
            return None;
        }
        let mut bound = source.clone();
        let mut resolved = false;
        for (effect, storage) in bound.effects.iter_mut().zip(&self.storage) {
            if effect.track != storage.track
                || storage.read_only
                || storage.preserve
                || storage.set_if.is_some()
            {
                return None;
            }
            let matching_from = match effect.from {
                QuestComparison::Eq(value) => storage.minimum == value && storage.maximum == value,
                QuestComparison::Between(minimum, maximum) => {
                    storage.minimum == minimum && storage.maximum == maximum
                }
                _ => false,
            };
            if !matching_from {
                return None;
            }
            if effect.effect == QuestEffectKind::Computed {
                if super::loader::inexact_source_set(&self.key, &storage.track) != Some(storage.set)
                {
                    return None;
                }
                effect.effect = QuestEffectKind::Set(storage.set);
                resolved = true;
            } else if effect.effect != QuestEffectKind::Set(storage.set) {
                return None;
            }
        }
        resolved.then_some(bound)
    }

    /// A pure quest-state predicate, with no Quest mutation or receipt.
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        !self.storage.is_empty() && self.storage.iter().all(|storage| storage.read_only)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    callbacks: Vec<Callback>,
}

/// Read the same closed content shape at boot and at the durable mutation boundary.
pub fn load() -> Result<Vec<Callback>, String> {
    let mut callbacks = parse(EMBEDDED)?;
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/source-world-callbacks.json"
    ))?);
    if callbacks
        .iter()
        .map(|callback| &callback.key)
        .collect::<BTreeSet<_>>()
        .len()
        != callbacks.len()
    {
        return Err("duplicate world callback across source shards".into());
    }
    callbacks.retain(|callback| super::in_runtime_scope(&callback.quest));
    if callbacks.len() >= MAX_CALLBACKS {
        return Err("callback bounds including Rotten Blood altar".into());
    }
    callbacks.push(super::rotten_blood::altar_callback());
    Ok(callbacks)
}

fn parse(text: &str) -> Result<Vec<Callback>, String> {
    let document: Document = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if document.schema != 1 || document.callbacks.len() > MAX_CALLBACKS {
        return Err("callback bounds".into());
    }
    let mut keys = BTreeSet::new();
    for callback in &document.callbacks {
        let selector = &callback.selector;
        let valid_pos = |p: Position| (-15..=0).contains(&p.floor);
        let unique_offerings: BTreeSet<_> = callback.offerings.iter().map(|o| o.position).collect();
        let unique_passages: BTreeSet<_> = callback.passages.iter().map(|p| p.position).collect();
        let unique_storage: BTreeSet<_> = callback.storage.iter().map(|s| &s.track).collect();
        if !super::valid_quest_key(&callback.key)
            || !super::valid_quest_key(&callback.quest)
            || !keys.insert(&callback.key)
            || (selector.position.is_none()
                && selector.unique.is_none()
                && selector.action.is_none()
                && selector.positions.is_empty())
            || selector.position.is_some_and(|p| !valid_pos(p))
            || selector.positions.len() > MAX_EFFECTS
            || selector.positions.iter().any(|p| !valid_pos(*p))
            || selector
                .positions
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != selector.positions.len()
            || (!selector.positions.is_empty()
                && (selector.position.is_some()
                    || selector.unique.is_some()
                    || selector.action.is_some()))
            || callback.required_claims.len() > MAX_EFFECTS
            || callback
                .required_claims
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != callback.required_claims.len()
            || callback
                .required_claims
                .iter()
                .any(|key| !super::valid_quest_key(key))
            || (!callback.required_claims.is_empty()
                && (callback.storage.is_empty() || callback.is_read_only()))
            || callback.teleport.is_some_and(|p| !valid_pos(p))
            || callback.failure_teleport.is_some_and(|p| {
                !valid_pos(p) || callback.trigger != Trigger::StepIn || !callback.return_on_failure
            })
            || callback.eject_occupants.as_ref().is_some_and(|ejection| {
                callback.trigger != Trigger::Use
                    || callback.objects.is_some()
                    || !callback.storage.is_empty()
                    || !callback.offerings.is_empty()
                    || callback.teleport.is_some()
                    || callback.item_on_target.is_some()
                    || callback.minimum_level.is_some()
                    || callback.experience.is_some()
                    || callback.monk_reward.is_some()
                    || callback.native_rule.is_some()
                    || ejection.positions.is_empty()
                    || ejection.positions.len() > MAX_EFFECTS
                    || ejection
                        .positions
                        .iter()
                        .copied()
                        .collect::<BTreeSet<_>>()
                        .len()
                        != ejection.positions.len()
                    || ejection.positions.iter().any(|p| !valid_pos(*p))
                    || !valid_pos(ejection.destination)
                    || ejection.positions.contains(&ejection.destination)
                    || callback
                        .passages
                        .iter()
                        .any(|p| p.position == ejection.destination)
            })
            || !objects::valid(callback)
            || unique_offerings.len() != callback.offerings.len()
            || unique_passages.len() != callback.passages.len()
            || unique_storage.len() != callback.storage.len()
            || (callback.storage.iter().any(|storage| storage.read_only)
                && (!callback.is_read_only()
                    || callback.trigger != Trigger::Use
                    || callback.teleport.is_none()
                    || callback.lever.is_some()
                    || !callback.passages.is_empty()
                    || !callback.offerings.is_empty()
                    || callback.item_on_target.is_some()
                    || callback.minimum_level.is_some()
                    || callback.experience.is_some()
                    || callback.monk_reward.is_some()
                    || callback.native_rule.is_some()
                    || callback.storage.iter().any(|storage| storage.preserve)))
            || callback.passages.len() > MAX_EFFECTS
            || callback.storage.len() > MAX_EFFECTS
            || callback
                .minimum_level
                .is_some_and(|level| level == 0 || callback.storage.is_empty())
            || callback.offerings.len() > MAX_EFFECTS
            || callback
                .experience
                .is_some_and(|xp| !(1..=super::QUESTGATE0_RL_05).contains(&xp))
            || callback.monk_reward.as_ref().is_some_and(|reward| {
                callback.quest != super::monk_shrines::QUEST
                    || callback.trigger != Trigger::Use
                    || callback.storage.is_empty()
                    || reward.item_source_id.is_some_and(|id| {
                        ![50267, 50271, 50269, 50273, 50274, 50272, 50268].contains(&id)
                    })
                    || reward.learn_spell.as_deref().is_some_and(|key| {
                        ![
                            "candidate:spell/mystic_repulse",
                            "candidate:spell/forceful_uppercut",
                            "candidate:spell/focus_harmony",
                        ]
                        .contains(&key)
                    })
            })
            || (!callback.offerings.is_empty() && callback.storage.is_empty())
            || callback.sources.is_empty()
            || !callback.sources.iter().any(|source| {
                // Dryad Garden uses the pinned Crystal donor; the earlier callbacks use Canary.
                source.starts_with("canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:")
                    || source.starts_with("crystal:201189f871d23597b82caf88fd611454a2ddefb8:")
                    || SOURCE_WORLD_REFS.contains(&source.as_str())
            })
            || callback.item_on_target.as_ref().is_some_and(|item| {
                item.target_replacement == Some(0)
                    || (item.replacement.is_some() && item.count != 1)
                    || callback.storage.is_empty()
                    || callback.trigger != Trigger::Use
                    || item.count == 0
                    || item.count > 100
                    || !super::valid_quest_key(&item.item)
                    || item
                        .replacement
                        .as_ref()
                        .is_some_and(|key| !super::valid_quest_key(key))
            })
            || callback
                .failure_message
                .as_ref()
                .is_some_and(|s| s.is_empty() || s.len() > 256 || s.contains(['\n', '\r', '\0']))
            || callback
                .success_message
                .as_ref()
                .is_some_and(|s| s.is_empty() || s.len() > 256 || s.contains(['\n', '\r', '\0']))
            || callback
                .native_rule
                .is_some_and(|rule| !valid_native_callback(callback, rule))
            || callback
                .lever
                .is_some_and(|p| {
                    p[0] == p[1]
                        || (callback.trigger != Trigger::Use
                            && (!callback.reversible
                                || !callback.storage.is_empty()
                                || callback.teleport.is_some()
                                || !callback.offerings.is_empty()
                                || callback.item_on_target.is_some()))
                })
            // A departure restores Channel map facts; it cannot write a second
            // durable consequence for the same committed movement command.
            || (callback.trigger == Trigger::StepOut
                && !callback.is_guarded_source_departure()
                && (!callback.reversible
                    || callback.teleport.is_some()
                    || !callback.storage.is_empty()
                    || !callback.offerings.is_empty()
                    || callback.item_on_target.is_some()
                    || callback.minimum_level.is_some()
                    || callback.experience.is_some()
                    || callback.monk_reward.is_some()
                    || callback.native_rule.is_some()
                    || callback.return_on_failure))
            || callback.passages.iter().any(|p| !valid_pos(p.position))
            || callback.offerings.iter().any(|o| {
                !valid_pos(o.position)
                    || o.count == 0
                    || o.count > 100
                    || !super::valid_quest_key(&o.item)
            })
            || callback.storage.iter().any(|s| {
                !super::valid_quest_key(&s.track)
                    || s.minimum > s.maximum
                    || s.set_if.is_some_and(|value| {
                        !(s.minimum..=s.maximum).contains(&value)
                            || s.preserve
                            || s.read_only
                            || callback.native_rule.is_some()
                    })
            })
        {
            return Err(format!("invalid callback {}", callback.key));
        }
    }
    Ok(document.callbacks)
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let callbacks = load()?;
    extend_callbacks(catalogue, &callbacks)
}

fn extend_callbacks(
    catalogue: QuestStateCatalogue,
    callbacks: &[Callback],
) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for callback in callbacks.iter().filter(|c| !c.storage.is_empty()) {
        if let Some(existing) = transitions
            .iter()
            .find(|transition| transition.key == callback.key)
        {
            let matching = existing.quest == callback.quest
                && !existing.completes
                && existing.experience == callback.experience
                && existing.effects.len() == callback.storage.len()
                && existing
                    .effects
                    .iter()
                    .zip(&callback.storage)
                    .all(|(effect, storage)| {
                        effect.track == storage.track
                            && !storage.preserve
                            && !storage.read_only
                            && callback.native_rule.is_none()
                            && (effect.effect == QuestEffectKind::Set(storage.set)
                                || (effect.effect == QuestEffectKind::Computed
                                    && !callback.required_claims.is_empty()
                                    && super::loader::inexact_source_set(
                                        &callback.key,
                                        &storage.track,
                                    ) == Some(storage.set)))
                            && match effect.from {
                                QuestComparison::Eq(value) => {
                                    storage.minimum == value && storage.maximum == value
                                }
                                QuestComparison::Between(minimum, maximum) => {
                                    storage.minimum == minimum && storage.maximum == maximum
                                }
                                _ => false,
                            }
                    });
            if !matching {
                return Err(format!(
                    "world binding differs from source transition {}",
                    callback.key
                ));
            }
            continue;
        }
        for storage in &callback.storage {
            if let Some(track) = tracks.iter_mut().find(|track| track.key == storage.track) {
                if callback.native_rule.is_none() && !storage.preserve && !storage.read_only {
                    track.max = track.max.max(storage.set).max(storage.maximum);
                }
            } else {
                if callback.native_rule.is_some() {
                    return Err(format!(
                        "native callback {} requires an existing track {}",
                        callback.key, storage.track
                    ));
                }
                tracks.push(QuestTrack {
                    key: storage.track.clone(),
                    quest: callback.quest.clone(),
                    initial: -1,
                    min: -1,
                    max: if storage.preserve || storage.read_only {
                        storage.maximum
                    } else {
                        storage.set.max(storage.maximum)
                    },
                });
            }
        }
        // Pure gates must not acquire a generic transition: even ADD(0)
        // creates a Character revision, quest state and receipt.
        if callback.is_read_only() {
            continue;
        }
        transitions.push(QuestTransition {
            key: callback.key.clone(),
            quest: callback.quest.clone(),
            effects: callback
                .storage
                .iter()
                .map(|s| QuestEffect {
                    track: s.track.clone(),
                    from: QuestComparison::Between(s.minimum, s.maximum),
                    effect: match callback.native_rule {
                        Some(rule) => QuestEffectKind::Native(rule),
                        None if s.preserve => QuestEffectKind::Add(0),
                        None => s.set_if.map_or(QuestEffectKind::Set(s.set), |equals| {
                            QuestEffectKind::SetIf {
                                equals,
                                value: s.set,
                            }
                        }),
                    },
                })
                .collect(),
            completes: false,
            experience: callback.experience,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod batch4_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditional_storage_compiles_exact_value_effect_and_rejects_guard_overloading()
    -> Result<(), String> {
        let callback = serde_json::json!({
            "key":"oteryn:quest-callback/conditional-reset", "quest":"oteryn:quest/conditional-reset",
            "trigger":"use", "selector":{"position":{"x":1,"y":2,"floor":-7}},
            "reversible":false, "passages":[], "offerings":[], "return_on_failure":false,
            "storage":[{"track":"oteryn:quest-progress/conditional-reset/flag", "minimum":-1,"maximum":2,"set":0,"set_if":1}],
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:fixture.lua:1"]
        });
        let encoded = |callback: serde_json::Value| {
            serde_json::json!({"schema":1,"callbacks":[callback]}).to_string()
        };
        let callbacks = parse(&encoded(callback.clone()))?;
        let catalogue = extend_callbacks(
            QuestStateCatalogue::empty("content-1").ok_or("catalogue")?,
            &callbacks,
        )?;
        let effect = catalogue
            .transition("oteryn:quest-callback/conditional-reset")
            .ok_or("transition")?
            .effects[0]
            .effect;
        assert_eq!(
            effect,
            QuestEffectKind::SetIf {
                equals: 1,
                value: 0
            }
        );
        for (field, value) in [
            ("set_if", serde_json::json!(3)),
            ("read_only", serde_json::json!(true)),
            ("preserve", serde_json::json!(true)),
        ] {
            let mut invalid = callback.clone();
            invalid["storage"][0][field] = value;
            assert!(parse(&encoded(invalid)).is_err());
        }
        Ok(())
    }

    #[test]
    fn conditional_failure_destination_requires_returning_step_and_native_floor()
    -> Result<(), String> {
        let document: serde_json::Value =
            serde_json::from_str(EMBEDDED).map_err(|e| e.to_string())?;
        let mut callback = document["callbacks"]
            .as_array()
            .ok_or("callbacks")?
            .iter()
            .find(|c| {
                c["key"]
                    .as_str()
                    .is_some_and(|k| k.ends_with("liquid-black-shortcut"))
            })
            .cloned()
            .ok_or("returning step")?;
        callback["failure_teleport"] = serde_json::json!({"x":32818,"y":31599,"floor":-9});
        let encoded = |callback: serde_json::Value| {
            serde_json::json!({"schema":1,"callbacks":[callback]}).to_string()
        };
        assert!(parse(&encoded(callback.clone())).is_ok());
        for (field, value) in [
            ("trigger", serde_json::json!("use")),
            ("return_on_failure", serde_json::json!(false)),
            (
                "failure_teleport",
                serde_json::json!({"x":1,"y":2,"floor":-16}),
            ),
        ] {
            let mut invalid = callback.clone();
            invalid[field] = value;
            assert!(parse(&encoded(invalid)).is_err());
        }
        Ok(())
    }

    #[test]
    fn departure_callbacks_restore_only_stateless_map_effects() -> Result<(), String> {
        let mut source: serde_json::Value =
            serde_json::from_str(EMBEDDED).map_err(|error| error.to_string())?;
        let mut callback = source["callbacks"]
            .as_array_mut()
            .ok_or("callbacks")?
            .iter()
            .find(|callback| callback["key"] == "oteryn:quest-callback/behemoth")
            .ok_or("behemoth")?
            .clone();
        callback["trigger"] = "step_out".into();
        callback["reversible"] = true.into();
        let document =
            |callback| serde_json::json!({"schema":1,"callbacks":[callback]}).to_string();
        assert!(parse(&document(callback.clone())).is_ok());
        for case in 0..5 {
            let mut invalid = callback.clone();
            match case {
                0 => invalid["reversible"] = false.into(),
                1 => {
                    invalid["storage"] = serde_json::json!([{
                        "track":"oteryn:quest-track/test", "minimum":-1, "maximum":1, "set":1
                    }])
                }
                2 => {
                    invalid["offerings"] = serde_json::json!([{
                        "position":{"x":33293,"y":31680,"floor":-15},
                        "item":"oteryn:item.tibia.i3031", "count":1
                    }])
                }
                3 => invalid["teleport"] = serde_json::json!({"x":33293,"y":31680,"floor":-15}),
                _ => invalid["return_on_failure"] = true.into(),
            }
            assert!(
                parse(&document(invalid)).is_err(),
                "departure restriction {case}"
            );
        }
        Ok(())
    }

    #[test]
    fn read_only_world_callback_has_no_quest_write_transition() -> Result<(), String> {
        let held: serde_json::Value = serde_json::from_str(include_str!("../../../../../tools/content-schema/quest-authoring/batch3_c/held-world/the_order_of_the_stag.json")).map_err(|e| e.to_string())?;
        let callbacks = parse(&serde_json::json!({"schema": 1, "callbacks": [held["callbacks"][1].clone(), held["callbacks"][3].clone()]}).to_string())?;
        let catalogue = extend_callbacks(
            QuestStateCatalogue::empty("content-1").ok_or("catalogue")?,
            &callbacks,
        )?;
        for callback in callbacks {
            assert!(
                catalogue.transition(&callback.key).is_none(),
                "read-only USE acquired a write transition"
            );
            assert!(catalogue.track(&callback.storage[0].track).is_some());
        }
        Ok(())
    }

    #[test]
    fn read_only_world_callbacks_reject_mixed_mutations() -> Result<(), String> {
        let held: serde_json::Value = serde_json::from_str(include_str!("../../../../../tools/content-schema/quest-authoring/batch3_c/held-world/the_order_of_the_stag.json")).map_err(|e| e.to_string())?;
        for case in 0..4 {
            let mut callback = held["callbacks"][1].clone();
            match case {
                0 => callback["experience"] = 1.into(),
                1 => callback["storage"][0]["preserve"] = true.into(),
                2 => callback["trigger"] = "step_in".into(),
                _ => callback["teleport"] = serde_json::Value::Null,
            }
            assert!(
                parse(&serde_json::json!({"schema": 1, "callbacks": [callback]}).to_string())
                    .is_err()
            );
        }
        Ok(())
    }

    #[test]
    fn dryad_garden_uses_source_unique_ids_in_both_directions() -> Result<(), String> {
        let callbacks = load()?;
        for (suffix, unique, destination) in [
            (
                "enter",
                33331,
                Position {
                    x: 33202,
                    y: 32012,
                    floor: -11,
                },
            ),
            (
                "leave",
                33332,
                Position {
                    x: 33264,
                    y: 32012,
                    floor: -7,
                },
            ),
        ] {
            let key = format!("oteryn:quest-callback/dryad-garden-{suffix}");
            let callback = callbacks.iter().find(|c| c.key == key).ok_or(key)?;
            assert_eq!(callback.trigger, Trigger::Use);
            assert_eq!(callback.selector.unique, Some(unique));
            assert_eq!(callback.teleport, Some(destination));
            assert!(callback.storage.is_empty());
            assert!(callback.sources.iter().all(|source| {
                source.starts_with("crystal:201189f871d23597b82caf88fd611454a2ddefb8:")
            }));
        }
        Ok(())
    }

    #[test]
    fn quest_world_callbacks_accept_only_pinned_donor_revisions() -> Result<(), String> {
        let mut source: serde_json::Value =
            serde_json::from_str(EMBEDDED).map_err(|e| e.to_string())?;
        source["callbacks"]
            .as_array_mut()
            .ok_or("callbacks")?
            .truncate(1);
        for (provenance, accepted) in [
            (
                "canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:script.lua:1",
                true,
            ),
            (
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:script.lua:1",
                true,
            ),
            ("crystal:unverified:script.lua:1", false),
            (
                "canary:201189f871d23597b82caf88fd611454a2ddefb8:script.lua:1",
                false,
            ),
            (
                "crystal:47dfd51f45280a59a1d3e50ba7edd573d7234446:script.lua:1",
                false,
            ),
            (
                "crystal:201189f871d23597b82caf88fd611454a2ddefb80:script.lua:1",
                false,
            ),
            ("tibiawiki:1203780", false),
        ] {
            source["callbacks"][0]["sources"] = serde_json::json!([provenance]);
            assert_eq!(parse(&source.to_string()).is_ok(), accepted, "{provenance}");
        }
        Ok(())
    }

    #[test]
    fn quest_world_callbacks_shortcut_preserves_the_source_storage_gate() -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        let c = load()?
            .into_iter()
            .find(|c| c.key.ends_with("liquid-black-shortcut"))
            .ok_or("shortcut")?;
        let catalogue = lowered.catalogue();
        let transition = catalogue.transition(&c.key).ok_or("transition")?;
        assert_eq!(
            catalogue.evaluate(transition, &Default::default(), 0),
            Err(super::super::QuestRefusal::StageMismatch)
        );
        for value in [4, 5] {
            let changes = catalogue
                .evaluate(
                    transition,
                    &std::collections::BTreeMap::from([(c.storage[0].track.clone(), value)]),
                    0,
                )
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!((changes[0].before, changes[0].after), (value, 5));
        }
        Ok(())
    }

    #[test]
    fn quest_world_callbacks_reject_ambiguous_and_unbounded_authoring() -> Result<(), String> {
        let source: serde_json::Value =
            serde_json::from_str(EMBEDDED).map_err(|e| e.to_string())?;
        let callbacks = source["callbacks"].as_array().ok_or("callbacks")?;
        let offerings = callbacks
            .iter()
            .position(|c| c["key"] == "oteryn:quest-callback/paradox-skulls")
            .ok_or("offerings callback")?;
        let item = callbacks
            .iter()
            .position(|c| c["key"] == "oteryn:quest-callback/fish-napping-key")
            .ok_or("item callback")?;
        for case in 0..10 {
            let mut bad = source.clone();
            match case {
                0 => {
                    bad["callbacks"][0]["selector"] = serde_json::json!({"position":null,"unique":null,"action":null,"appearance":null})
                }
                1 => bad["callbacks"][0]["extra"] = true.into(),
                2 => {
                    bad["callbacks"][offerings]["offerings"][1] =
                        bad["callbacks"][offerings]["offerings"][0].clone()
                }
                3 => bad["callbacks"][offerings]["offerings"][0]["count"] = 101.into(),
                4 => bad["callbacks"][item]["item_on_target"]["target_replacement"] = 0.into(),
                5 => bad["callbacks"][0]["sources"] = serde_json::json!(["crystal:unverified"]),
                6 => bad["callbacks"][item]["item_on_target"]["replacement"] = "bad".into(),
                7 => {
                    bad["callbacks"][item]["item_on_target"]["replacement"] =
                        "oteryn:item.tibia.i3114".into();
                    bad["callbacks"][item]["item_on_target"]["count"] = 2.into();
                }
                8 => bad["callbacks"][item]["storage"] = serde_json::json!([]),
                _ => bad["callbacks"][0]["failure_message"] = "x".repeat(257).into(),
            }
            assert!(parse(&bad.to_string()).is_err(), "case {case}");
        }
        let mut bounded = source.clone();
        bounded["callbacks"] = serde_json::json!([]);
        for index in 0..MAX_CALLBACKS {
            let mut callback = source["callbacks"][0].clone();
            callback["key"] = format!("oteryn:quest-callback/bounded-{index}").into();
            bounded["callbacks"]
                .as_array_mut()
                .ok_or("callbacks")?
                .push(callback);
        }
        assert!(parse(&bounded.to_string()).is_ok());
        let mut extra = source["callbacks"][0].clone();
        extra["key"] = "oteryn:quest-callback/overflow".into();
        bounded["callbacks"]
            .as_array_mut()
            .ok_or("callbacks")?
            .push(extra);
        assert!(parse(&bounded.to_string()).is_err());
        for field in ["passages", "storage", "offerings"] {
            let mut bounded = source.clone();
            let entries = (0..MAX_EFFECTS).map(|index| match field {
                "passages" => serde_json::json!({"position":{"x":index,"y":1,"floor":-1},"appearance":1791}),
                "storage" => serde_json::json!({"track":format!("oteryn:quest-progress/bounded-{index}"),"minimum":-1,"maximum":1,"set":1}),
                _ => serde_json::json!({"position":{"x":index,"y":1,"floor":-1},"item":"oteryn:item.tibia.i3114","count":100}),
            }).collect::<Vec<_>>();
            bounded["callbacks"][offerings][field] = serde_json::json!(entries);
            assert!(parse(&bounded.to_string()).is_ok(), "{field} at its bound");
            let mut extra = bounded["callbacks"][offerings][field][0].clone();
            if field == "storage" {
                extra["track"] = "oteryn:quest-progress/overflow".into();
            } else {
                extra["position"]["x"] = 100.into();
            }
            bounded["callbacks"][offerings][field]
                .as_array_mut()
                .ok_or("entries")?
                .push(extra);
            assert!(
                parse(&bounded.to_string()).is_err(),
                "{field} past its bound"
            );
        }
        let mut bounded = source;
        bounded["callbacks"][0]["failure_message"] = "x".repeat(256).into();
        assert!(parse(&bounded.to_string()).is_ok());
        Ok(())
    }

    #[test]
    fn quest_world_callbacks_three_bounded_action_classes_are_source_witnessed()
    -> Result<(), String> {
        let callbacks = load()?;
        let lowered = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        for callback in callbacks.iter().filter(|c| {
            c.item_on_target.is_some()
                || c.key.ends_with("outlaw-camp-door")
                || c.key.ends_with("travelling-trader-delivery")
        }) {
            for storage in &callback.storage {
                assert!(
                    lowered.source_track(&storage.track).is_some(),
                    "{}",
                    storage.track
                );
                assert_eq!(
                    lowered
                        .catalogue()
                        .track(&storage.track)
                        .ok_or("track")?
                        .quest,
                    callback.quest
                );
            }
        }
        let outlaw = callbacks
            .iter()
            .find(|callback| callback.key.ends_with("outlaw-camp-door"))
            .ok_or("outlaw door")?;
        assert_eq!(
            (outlaw.selector.action, outlaw.lever),
            (Some(1108), Some([1642, 1643]))
        );
        assert_eq!(
            (outlaw.storage[0].minimum, outlaw.storage[0].maximum),
            (3, 3)
        );

        let key = callbacks
            .iter()
            .find(|callback| callback.key.ends_with("fish-napping-key"))
            .ok_or("fish key")?;
        let item = key.item_on_target.as_ref().ok_or("item target")?;
        assert_eq!((&*item.item, item.count), ("oteryn:item.tibia.i7934", 1));
        assert_eq!(key.selector.action, Some(51394));

        let door = callbacks
            .iter()
            .find(|callback| callback.key.ends_with("travelling-trader-delivery"))
            .ok_or("trader delivery")?;
        assert_eq!(
            (
                door.storage[0].minimum,
                door.storage[0].maximum,
                door.storage[0].set
            ),
            (3, 3, 4)
        );
        assert_eq!(door.selector.unique, Some(3020));
        Ok(())
    }

    /// PAR-W15: lever/step callbacks against Crystal `201189f8`
    /// (`quests/behemoth/action-lever.lua`, `others/actions_hellgate_lever.lua`,
    /// `liquid_black/movements_shortcut.lua`). Oteryn floors are stored as `-z`.
    #[test]
    fn w15_crystal_world_callback_parity() -> Result<(), String> {
        let callbacks = load()?;
        let find = |suffix: &str| {
            callbacks
                .iter()
                .find(|c| c.key.ends_with(suffix))
                .ok_or(suffix.to_string())
        };
        let row = |x: u16, y: u16, floor: i8, n: u16| -> Vec<Position> {
            (0..n).map(|i| Position { x: x + i, y, floor }).collect()
        };
        // Behemoth: uid 30009, one-way lever 2772 -> 2773, removes wall 1791 at
        // 33295..=33299, 31677, 15. MATCH.
        let behemoth = find("behemoth")?;
        assert_eq!(behemoth.trigger, Trigger::Use);
        assert_eq!(behemoth.selector.unique, Some(30009));
        assert_eq!(
            (behemoth.lever, behemoth.reversible),
            (Some([2772, 2773]), false)
        );
        assert_eq!(
            behemoth
                .passages
                .iter()
                .map(|p| p.position)
                .collect::<Vec<_>>(),
            row(33295, 31677, -15, 5)
        );
        assert!(
            behemoth
                .passages
                .iter()
                .all(|p| p.appearance == 1791 && p.replacement.is_none())
        );
        // Hellgate bridge: aid 50027, reversible lever, water 622 <-> bridge 5770 at
        // 32627..=32629, 31699, 10. MATCH. DIFF: Crystal also moves a creature standing on
        // the bridge to 32630, 31699, 10; the Oteryn record has no creature push.
        let hellgate = find("hellgate-bridge")?;
        assert_eq!(hellgate.selector.action, Some(50027));
        assert_eq!(
            (hellgate.lever, hellgate.reversible),
            (Some([2772, 2773]), true)
        );
        assert_eq!(
            hellgate
                .passages
                .iter()
                .map(|p| p.position)
                .collect::<Vec<_>>(),
            row(32627, 31699, -10, 3)
        );
        assert!(
            hellgate
                .passages
                .iter()
                .all(|p| p.appearance == 622 && p.replacement == Some(5770))
        );
        assert_eq!(hellgate.teleport, None);
        // Liquid Black shortcut: aid 57746 step-in, Visitor >= 4 sets 5 and teleports to
        // 33478, 31314, 7, otherwise pushes back. MATCH (the track's maximum is 5).
        let shortcut = find("liquid-black-shortcut")?;
        assert_eq!(shortcut.trigger, Trigger::StepIn);
        assert_eq!(shortcut.selector.action, Some(57746));
        assert_eq!(
            shortcut.teleport,
            Some(Position {
                x: 33478,
                y: 31314,
                floor: -7
            })
        );
        assert_eq!(
            (
                shortcut.storage[0].minimum,
                shortcut.storage[0].set,
                shortcut.return_on_failure
            ),
            (4, 5, true)
        );
        // MISSING: the Annihilator lever (Crystal `annihilator.lua`, uid 30015).
        assert!(callbacks.iter().all(|c| c.selector.unique != Some(30015)));
        Ok(())
    }
}
