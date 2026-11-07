//! QUESTCB2: bounded, declarative world callbacks. Coordinates use native negative floors.
pub use super::native_rules::NativeRule;
mod carried;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
pub use carried::{ActorRegion, CarriedUse, TimeWindow};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const EMBEDDED: &str = include_str!("../../../../../content/quests/world-callbacks.json");
// Static quest batches and the code-built source callbacks share this bound.
pub const MAX_CALLBACKS: usize = 512;
pub const MAX_EFFECTS: usize = 8;

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
    UseCarried,
    UseCarriedOnTarget,
}

mod effects;
pub use effects::{
    CallbackEffects, Encounter, EncounterMonster, EncounterPlayer, Facing, ItemCycle, MagicEffect,
    Portal, PortalPair,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    pub position: Option<Position>,
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

/// Read one other effect's original locked value; reuse the quest engine's conditional set.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetWhenAtLeast {
    pub other_effect: u8,
    pub minimum: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Storage {
    pub track: String,
    pub minimum: i64,
    pub maximum: i64,
    pub set: i64,
    /// Normal world callbacks may add to the locked value instead of setting it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub increment: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_when_at_least: Option<SetWhenAtLeast>,
    /// A read-only gate: validate the locked value without resetting later progress.
    #[serde(default, skip_serializing_if = "is_false")]
    pub preserve: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub(super) fn source_backend(source: &str) -> Option<&'static str> {
    if source.starts_with("crystal:201189f871d23597b82caf88fd611454a2ddefb8:") {
        Some("crystalserver")
    } else if source.starts_with("canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:") {
        Some("canary")
    } else {
        None
    }
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MonkReward {
    pub item_source_id: Option<u32>,
    pub learn_spell: Option<String>,
    #[serde(default)]
    pub final_shrine: bool,
}

/// Bounded native reward output; absent effects retain every published binding.
/// A source object returns to its admitted appearance after a bounded Channel deadline.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TemporaryTransform {
    pub appearance: u16,
    pub restore: u16,
    pub lifetime_us: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Callback {
    /// Each earned state cycle gets an independent native MINT claim.
    #[serde(default, skip_serializing_if = "is_false")]
    pub repeatable_reward: bool,
    pub key: String,
    pub quest: String,
    pub trigger: Trigger,
    pub selector: Selector,
    /// Both lever appearances. None for a step tile. One-way levers refuse their second use.
    pub lever: Option<[u16; 2]>,
    pub reversible: bool,
    pub passages: Vec<Passage>,
    pub teleport: Option<Position>,
    pub storage: Vec<Storage>,
    /// Checked against the progression owner in the same fenced quest transaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_change: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monk_reward: Option<MonkReward>,
    pub offerings: Vec<Offering>,
    #[serde(default)]
    pub item_on_target: Option<ItemOnTarget>,
    pub return_on_failure: bool,
    pub failure_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_rule: Option<NativeRule>,
    #[serde(default, skip_serializing_if = "CallbackEffects::is_empty")]
    pub effects: CallbackEffects,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carried_use: Option<CarriedUse>,
    pub sources: Vec<String>,
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
        "../../../../../content/quests/shared-access-callbacks.json"
    ))?);
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/shared-outfit-callbacks.json"
    ))?);
    if callbacks.len() > MAX_CALLBACKS.saturating_sub(2) {
        return Err("callback bounds including native source callbacks".into());
    }
    callbacks.push(super::rotten_blood::altar_callback());
    callbacks.push(super::schrodinger_island::callback());
    let keys: BTreeSet<_> = callbacks.iter().map(|c| &c.key).collect();
    if keys.len() != callbacks.len() {
        return Err("duplicate callback across registries".into());
    }
    Ok(callbacks)
}

fn valid_carried(callback: &Callback) -> bool {
    let Some(rule) = &callback.carried_use else {
        return !matches!(
            callback.trigger,
            Trigger::UseCarried | Trigger::UseCarriedOnTarget
        );
    };
    matches!(
        callback.trigger,
        Trigger::UseCarried | Trigger::UseCarriedOnTarget
    ) && rule.actor_region.valid()
        && rule.time_window.valid()
        && (callback.trigger == Trigger::UseCarriedOnTarget
            || (callback.selector.position.is_none()
                && callback.selector.unique.is_none()
                && callback.selector.action.is_none()
                && callback.selector.appearance.is_none()))
        && callback.lever.is_none()
        && !callback.reversible
        && callback.passages.is_empty()
        && callback.offerings.is_empty()
        && callback.teleport.is_none()
        && callback.native_rule.is_none()
        && callback.experience.is_none()
        && callback.monk_reward.is_none()
        && callback.effects.temporary_transform.is_none()
        && (callback.effects.item_reward.is_none()
            || (callback.trigger == Trigger::UseCarriedOnTarget
                && callback
                    .item_on_target
                    .as_ref()
                    .is_some_and(|item| item.replacement.is_none())
                && callback
                    .storage
                    .iter()
                    .any(|storage| !storage.preserve && storage.increment.is_none())))
        && !callback.return_on_failure
        && !callback.storage.is_empty()
        && callback.item_on_target.as_ref().is_some_and(|item| {
            item.count == 1
                && item.target_replacement.is_none()
                && (callback.trigger == Trigger::UseCarriedOnTarget
                    || item
                        .replacement
                        .as_ref()
                        .is_some_and(|replacement| replacement != &item.item))
        })
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
            || (callback.trigger != Trigger::UseCarried
                && selector.position.is_none()
                && selector.unique.is_none()
                && selector.action.is_none()
                && (!matches!(callback.trigger, Trigger::Use | Trigger::UseCarriedOnTarget)
                    || selector.appearance.is_none_or(|appearance| appearance == 0)))
            || !valid_carried(callback)
            || callback
                .effects
                .temporary_transform
                .as_ref()
                .is_some_and(|effect| {
                    effect.appearance == 0
                        || effect.restore == 0
                        || effect.appearance == effect.restore
                        || !(1..=600_000_000).contains(&effect.lifetime_us)
                        || callback.trigger != Trigger::Use
                        || callback.storage.is_empty()
                        || callback.effects.item_reward.is_some()
                        || callback.lever.is_some()
                        || callback.reversible
                        || !callback.passages.is_empty()
                        || callback.teleport.is_some()
                        || !callback.offerings.is_empty()
                        || callback.item_on_target.is_some()
                        || callback.carried_use.is_some()
                        || callback.native_rule.is_some()
                        || callback.monk_reward.is_some()
                        || callback.experience.is_some()
                })
            || callback.storage.iter().any(|storage| {
                storage.increment.is_some_and(|delta| {
                    delta == 0
                        || !(-1_000_000..=1_000_000).contains(&delta)
                        || storage.preserve
                        || (callback.carried_use.is_some()
                            && !callback
                                .storage
                                .iter()
                                .any(|bound| !bound.preserve && bound.increment.is_none()))
                        || callback.native_rule.is_some()
                        || storage.minimum.checked_add(delta).is_none()
                        || storage.maximum.checked_add(delta).is_none()
                })
            })
            || callback.storage.iter().enumerate().any(|(index, storage)| {
                storage.set_when_at_least.is_some_and(|condition| {
                    storage.preserve
                        || storage.increment.is_some()
                        || callback.native_rule.is_some()
                        || callback.carried_use.is_some()
                        || usize::from(condition.other_effect) >= callback.storage.len()
                        || usize::from(condition.other_effect) == index
                })
            })
            || (callback.repeatable_reward
                && (callback.effects.item_reward.is_none()
                    || !callback
                        .storage
                        .iter()
                        .any(|s| !s.preserve && !(s.minimum..=s.maximum).contains(&s.set))))
            || selector.position.is_some_and(|p| !valid_pos(p))
            || callback.teleport.is_some_and(|p| !valid_pos(p))
            || !effects::valid(callback)
            || unique_offerings.len() != callback.offerings.len()
            || unique_passages.len() != callback.passages.len()
            || unique_storage.len() != callback.storage.len()
            || callback.passages.len() > MAX_EFFECTS
            || callback.storage.len() > MAX_EFFECTS
            || callback
                .minimum_level
                .is_some_and(|level| level == 0 || callback.storage.is_empty())
            || callback.effects.item_reward.as_ref().is_some_and(|reward| {
                !super::reward_callbacks::valid_item_reward(reward)
                    || !matches!(
                        callback.trigger,
                        Trigger::Use | Trigger::StepIn | Trigger::UseCarriedOnTarget
                    )
                    || callback.storage.is_empty()
                    || callback.reversible
                    || callback.lever.is_some()
                    || !callback.passages.is_empty()
                    || callback.teleport.is_some()
                    || !callback.offerings.is_empty()
                    || (callback.trigger != Trigger::UseCarriedOnTarget
                        && callback.item_on_target.is_some())
                    || (callback.trigger != Trigger::UseCarriedOnTarget
                        && callback.carried_use.is_some())
                    || callback.native_rule.is_some()
                    || callback.monk_reward.is_some()
                    || callback.experience.is_some()
            })
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
                source_backend(source).is_some()
                    || (callback.effects.outfit_reward.is_some()
                        && source.starts_with("crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:"))
            })
            || callback.item_on_target.as_ref().is_some_and(|item| {
                item.target_replacement == Some(0)
                    || (item.replacement.is_some() && item.count != 1)
                    || callback.storage.is_empty()
                    || !matches!(
                        callback.trigger,
                        Trigger::Use | Trigger::UseCarried | Trigger::UseCarriedOnTarget
                    )
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
                .is_some_and(|p| p[0] == p[1] || callback.trigger != Trigger::Use)
            || callback.passages.iter().any(|p| !valid_pos(p.position))
            || callback.offerings.iter().any(|o| {
                !valid_pos(o.position)
                    || o.count == 0
                    || o.count > 100
                    || !super::valid_quest_key(&o.item)
            })
            || callback
                .storage
                .iter()
                .any(|s| !super::valid_quest_key(&s.track) || s.minimum > s.maximum)
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
        let carried_binding: Option<[u8; 32]> = callback
            .carried_use
            .as_ref()
            .map(|_| {
                let bytes = serde_json::to_vec(&(
                    &callback.key,
                    &callback.quest,
                    callback.trigger,
                    &callback.storage,
                    &callback.item_on_target,
                    &callback.carried_use,
                ))
                .map_err(|error| error.to_string())?;
                let mut digest = Sha256::new();
                digest.update(bytes);
                if callback.trigger == Trigger::UseCarriedOnTarget {
                    digest.update(b"\0quest-carried-target-selector-v1\0");
                    digest.update(
                        serde_json::to_vec(&callback.selector)
                            .map_err(|error| error.to_string())?,
                    );
                }
                if let Some(level) = callback.minimum_level {
                    digest.update(b"\0quest-carried-minimum-level-v1\0");
                    digest.update(level.to_be_bytes());
                }
                if let Some(reward) = &callback.effects.item_reward {
                    digest.update(b"\0quest-carried-target-item-reward-v1\0");
                    digest.update(serde_json::to_vec(reward).map_err(|error| error.to_string())?);
                }
                Ok::<[u8; 32], String>(digest.finalize().into())
            })
            .transpose()?;
        for storage in &callback.storage {
            let increment_min = storage
                .increment
                .and_then(|delta| storage.minimum.checked_add(delta));
            let increment_max = storage
                .increment
                .and_then(|delta| storage.maximum.checked_add(delta));
            if let Some(track) = tracks.iter_mut().find(|track| track.key == storage.track) {
                if callback.native_rule.is_none() && !storage.preserve {
                    track.max = track
                        .max
                        .max(storage.set)
                        .max(storage.maximum)
                        .max(increment_max.unwrap_or(i64::MIN));
                    track.min = track.min.min(increment_min.unwrap_or(i64::MAX));
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
                    min: (-1).min(increment_min.unwrap_or(i64::MAX)),
                    max: if storage.preserve {
                        storage.maximum
                    } else {
                        storage
                            .set
                            .max(storage.maximum)
                            .max(increment_max.unwrap_or(i64::MIN))
                    },
                });
            }
        }
        transitions.push(QuestTransition {
            world_change: callback.world_change.clone(),
            key: callback.key.clone(),
            quest: callback.quest.clone(),
            effects: callback
                .storage
                .iter()
                .map(|s| QuestEffect {
                    track: s.track.clone(),
                    from: QuestComparison::Between(s.minimum, s.maximum),
                    effect: match (callback.native_rule, s.set_when_at_least) {
                        (Some(rule), _) => QuestEffectKind::Native(rule),
                        (None, _) if s.preserve => QuestEffectKind::Add(0),
                        (None, Some(condition)) => QuestEffectKind::SetWhenAtLeast {
                            other_effect: condition.other_effect,
                            minimum: condition.minimum,
                            value: s.set,
                        },
                        (None, None) if s.increment.is_some() => {
                            QuestEffectKind::Add(s.increment.unwrap_or(0))
                        }
                        (None, None) => match carried_binding {
                            Some(binding) => QuestEffectKind::SetWithBinding {
                                value: s.set,
                                binding,
                            },
                            None => QuestEffectKind::Set(s.set),
                        },
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
    fn qengine_shared_access_rejects_ambiguous_or_unbounded_effects() -> Result<(), String> {
        let source: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/shared-access-callbacks.json"
        ))
        .map_err(|e| e.to_string())?;
        parse(&source.to_string())?;
        let callbacks = source["callbacks"].as_array().ok_or("callbacks")?;
        let index = |name: &str| {
            callbacks
                .iter()
                .position(|c| c["key"].as_str().is_some_and(|k| k.ends_with(name)))
                .ok_or(name.to_owned())
        };
        let portals = index("thais-lighthouse-portals")?;
        let encounter = index("annihilator-entry")?;
        let cycle = index("triangle-tower-lever")?;
        for case in 0..8 {
            let mut bad = source.clone();
            match case {
                0 => {
                    bad["callbacks"][portals]["effects"]["portals"]["portals"][1]["position"] = bad
                        ["callbacks"][portals]["effects"]["portals"]["portals"][0]["position"]
                        .clone()
                }
                1 => bad["callbacks"][portals]["effects"]["portals"]["lifetime_us"] = 0.into(),
                2 => {
                    bad["callbacks"][encounter]["effects"]["encounter"]["players"] =
                        serde_json::json!([])
                }
                3 => {
                    bad["callbacks"][encounter]["effects"]["encounter"]["players"][1]["to"] =
                        bad["callbacks"][encounter]["effects"]["encounter"]["players"][0]["to"]
                            .clone()
                }
                4 => {
                    bad["callbacks"][encounter]["effects"]["encounter"]["monsters"][0]["position"]
                        ["floor"] = 1.into()
                }
                5 => bad["callbacks"][cycle]["effects"]["item_cycle"]["create"] = 0.into(),
                6 => bad["callbacks"][cycle]["effects"]["facing"] = "north".into(),
                _ => bad["callbacks"][cycle]["effects"]["unknown"] = true.into(),
            }
            assert!(
                parse(&bad.to_string()).is_err(),
                "invalid effect case {case}"
            );
        }
        Ok(())
    }

    #[test]
    fn relative_world_counter_is_locked_bounded_and_requires_a_distinct_once_flag()
    -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|c| c.effects.temporary_transform.is_some())
            .ok_or("timed source callback")?;
        let tracks = callback
            .storage
            .iter()
            .map(|s| QuestTrack {
                key: s.track.clone(),
                quest: callback.quest.clone(),
                initial: 0,
                min: -1,
                max: s.maximum.max(s.set),
            })
            .collect();
        let base = QuestStateCatalogue::new("fixture", tracks, Vec::new())
            .map_err(|e| format!("{e:?}"))?;
        let catalogue = extend_callbacks(base, std::slice::from_ref(&callback))?;
        let transition = catalogue.transition(&callback.key).ok_or("transition")?;
        let mut stored = std::collections::BTreeMap::new();
        for storage in &callback.storage {
            stored.insert(storage.track.clone(), storage.minimum.max(0));
        }
        let counter = callback
            .storage
            .iter()
            .find(|s| s.increment.is_some())
            .ok_or("counter")?;
        stored.insert(counter.track.clone(), 4);
        let changes = catalogue
            .evaluate(transition, &stored, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            changes
                .iter()
                .find(|c| c.track == counter.track)
                .ok_or("counter change")?
                .after,
            5
        );
        stored.insert(counter.track.clone(), 5);
        assert!(
            catalogue.evaluate(transition, &stored, 0).is_err(),
            "the sixth release cannot pass the locked source counter gate"
        );
        stored.insert(counter.track.clone(), 0);
        let once = callback
            .storage
            .iter()
            .find(|s| !s.preserve && s.increment.is_none())
            .ok_or("once flag")?;
        stored.insert(once.track.clone(), 1);
        assert!(
            catalogue.evaluate(transition, &stored, 0).is_err(),
            "the same fairy cannot increment twice"
        );
        for mutation in 0..4 {
            let mut bad = callback.clone();
            match mutation {
                0 => {
                    bad.effects
                        .temporary_transform
                        .as_mut()
                        .ok_or("effect")?
                        .lifetime_us = 0
                }
                1 => {
                    bad.storage
                        .iter_mut()
                        .find(|s| s.increment.is_some())
                        .ok_or("counter")?
                        .increment = Some(0)
                }
                2 => {
                    bad.storage
                        .iter_mut()
                        .find(|s| s.increment.is_some())
                        .ok_or("counter")?
                        .preserve = true
                }
                _ => {
                    let c = bad
                        .storage
                        .iter_mut()
                        .find(|s| s.increment.is_some())
                        .ok_or("counter")?;
                    c.minimum = i64::MAX;
                    c.maximum = i64::MAX;
                }
            }
            assert!(
                parse(&serde_json::json!({"schema":1,"callbacks":[bad]}).to_string()).is_err(),
                "invalid relative/timer mutation {mutation}"
            );
        }
        Ok(())
    }

    #[test]
    fn repeatable_world_gifts_require_an_earned_state_cycle() -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|c| c.repeatable_reward)
            .ok_or("repeatable world gift")?;
        let document = |c: &Callback| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
        assert!(parse(&document(&callback)).is_ok());
        let mut free = callback.clone();
        for storage in &mut free.storage {
            storage.set = storage.minimum;
        }
        assert!(
            parse(&document(&free)).is_err(),
            "a repeat must leave its eligible state range"
        );
        let mut valueless = callback.clone();
        valueless.effects = Default::default();
        assert!(parse(&document(&valueless)).is_err());
        let mut empty_selector = callback;
        empty_selector.selector.position = None;
        empty_selector.selector.unique = None;
        empty_selector.selector.action = None;
        empty_selector.selector.appearance = Some(0);
        assert!(
            parse(&document(&empty_selector)).is_err(),
            "appearance-only USE needs a positive source item"
        );
        Ok(())
    }

    #[test]
    fn carried_target_rewards_bind_outputs_and_require_a_bound_once_write() -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|c| c.trigger == Trigger::UseCarriedOnTarget && c.effects.item_reward.is_some())
            .ok_or("carried target reward")?;
        let document = |c: &Callback| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
        assert!(parse(&document(&callback)).is_ok());
        let catalogue =
            || QuestStateCatalogue::new("test", vec![], vec![]).map_err(|e| format!("{e:?}"));
        let original = extend_callbacks(catalogue()?, std::slice::from_ref(&callback))?;
        let mut changed = callback.clone();
        changed.effects.item_reward.as_mut().ok_or("reward")?.count = 2;
        let changed_catalogue = extend_callbacks(catalogue()?, std::slice::from_ref(&changed))?;
        assert_ne!(
            original.transition(&callback.key),
            changed_catalogue.transition(&callback.key),
            "a reward change must change the carried callback binding"
        );
        for mode in 0..5 {
            let mut bad = callback.clone();
            match mode {
                0 => bad
                    .storage
                    .retain(|storage| storage.preserve || storage.increment.is_some()),
                1 => {
                    bad.item_on_target.as_mut().ok_or("input")?.replacement =
                        Some("oteryn:item.tibia.i25303".into())
                }
                2 => bad.carried_use = None,
                3 => bad.effects.item_reward.as_mut().ok_or("reward")?.count = 0,
                _ => bad.item_on_target.as_mut().ok_or("input")?.count = 2,
            }
            assert!(
                parse(&document(&bad)).is_err(),
                "unsafe carried reward mode{mode}"
            );
        }
        Ok(())
    }

    #[test]
    fn step_reward_conditional_storage_uses_the_original_locked_counter() -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|c| c.storage.iter().any(|s| s.set_when_at_least.is_some()))
            .ok_or("conditional source")?;
        let tracks = callback
            .storage
            .iter()
            .map(|s| QuestTrack {
                key: s.track.clone(),
                quest: callback.quest.clone(),
                initial: 0,
                min: -1,
                max: s.maximum.max(s.set),
            })
            .collect();
        let base =
            QuestStateCatalogue::new("fixture", tracks, vec![]).map_err(|e| format!("{e:?}"))?;
        let catalogue = extend_callbacks(base, std::slice::from_ref(&callback))?;
        let transition = catalogue.transition(&callback.key).ok_or("transition")?;
        let stage = &callback.storage[0].track;
        let counter = &callback.storage[1].track;
        let flag = &callback.storage[2].track;
        let mut stored = std::collections::BTreeMap::from([
            (stage.clone(), 13),
            (counter.clone(), 0),
            (flag.clone(), -1),
        ]);
        for before in 0..=4 {
            stored.insert(counter.clone(), before);
            let changes = catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                changes
                    .iter()
                    .find(|c| &c.track == counter)
                    .ok_or("counter")?
                    .after,
                before + 1
            );
            assert_eq!(
                changes
                    .iter()
                    .find(|c| &c.track == stage)
                    .ok_or("stage")?
                    .after,
                if before == 4 { 14 } else { 13 }
            );
            assert_eq!(
                changes
                    .iter()
                    .find(|c| &c.track == flag)
                    .ok_or("flag")?
                    .after,
                1
            );
        }
        stored.insert(counter.clone(), 5);
        assert!(
            catalogue.evaluate(transition, &stored, 0).is_err(),
            "cap gate protects the sixth grant"
        );
        stored.insert(counter.clone(), 0);
        stored.insert(flag.clone(), 1);
        assert!(
            catalogue.evaluate(transition, &stored, 0).is_err(),
            "same tile cannot grant twice"
        );
        for mode in 0..4 {
            let mut bad = callback.clone();
            match mode {
                0 => {
                    bad.storage[0].set_when_at_least = Some(SetWhenAtLeast {
                        other_effect: 0,
                        minimum: 4,
                    })
                }
                1 => {
                    bad.storage[0].set_when_at_least = Some(SetWhenAtLeast {
                        other_effect: 255,
                        minimum: 4,
                    })
                }
                2 => bad.storage[0].increment = Some(1),
                _ => bad.storage[0].preserve = true,
            }
            assert!(
                parse(&serde_json::json!({"schema":1,"callbacks":[bad]}).to_string()).is_err(),
                "conditional mode{mode} must refuse"
            );
        }
        Ok(())
    }

    #[test]
    fn world_item_rewards_are_bounded_and_cannot_enter_carried_or_unfenced_modes()
    -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|c| c.effects.item_reward.is_some())
            .ok_or("world reward")?;
        let document = |c: &Callback| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
        assert!(parse(&document(&callback)).is_ok());
        let mut absent = callback.clone();
        absent.effects = Default::default();
        assert!(
            serde_json::to_value(&absent)
                .map_err(|e| e.to_string())?
                .get("effects")
                .is_none(),
            "legacy binding must omit empty effects"
        );
        for mode in 0..8 {
            let mut bad = callback.clone();
            match mode {
                0 => {
                    // Plain StepIn rewards now use the complete MINT owner, but
                    // selected carried inputs still require native USE_WITH.
                    bad.trigger = Trigger::StepIn;
                    bad.item_on_target = Some(ItemOnTarget {
                        item: "oteryn:item.tibia.i25244".into(),
                        count: 1,
                        replacement: None,
                        target_replacement: None,
                    });
                }
                1 => bad.trigger = Trigger::UseCarried,
                2 => bad.trigger = Trigger::UseCarriedOnTarget,
                3 => bad.storage.clear(),
                4 => bad.reversible = true,
                5 => bad.effects.item_reward.as_mut().ok_or("reward")?.count = 0,
                6 => bad.effects.item_reward.as_mut().ok_or("reward")?.count = 101,
                _ => {
                    bad.effects.item_reward.as_mut().ok_or("reward")?.item =
                        "unqualified:item".into()
                }
            }
            assert!(
                parse(&document(&bad)).is_err(),
                "world reward mode{mode} must refuse"
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
    #[test]
    fn qengine_l1_world_outfit_sources_are_narrowly_admitted() -> Result<(), String> {
        let source = include_str!("../../../../../content/quests/shared-outfit-callbacks.json");
        assert_eq!(parse(source)?.len(), 2);
        for edit in 0..3 {
            let mut value: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            match edit {
                0 => value["callbacks"][0]["effects"]["outfit_reward"] = serde_json::Value::Null,
                1 => value["callbacks"][0]["effects"]["outfit_reward"]["addons"] = 4.into(),
                _ => {
                    for state in value["callbacks"][0]["storage"]
                        .as_array_mut()
                        .ok_or("storage")?
                    {
                        state["set"] = state["minimum"].clone();
                    }
                }
            }
            assert!(
                parse(&value.to_string()).is_err(),
                "invalid wardrobe edit {edit}"
            );
        }
        Ok(())
    }
}
