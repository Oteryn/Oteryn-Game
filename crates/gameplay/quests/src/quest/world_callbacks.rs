//! QUESTCB2: bounded, declarative world callbacks. Coordinates use native negative floors.
pub use super::native_rules::NativeRule;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
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
}

mod clock;
mod counterparts;
mod effects;
mod ground_stack;
mod imported_explore;
pub use effects::{
    CallbackEffectSet, CallbackEffects, Encounter, EncounterMonster, EncounterPlayer,
    EncounterTiming, Facing, ItemCycle, ItemPresence, MagicEffect, MountReward, Portal, PortalPair,
};
pub use ground_stack::{GroundStackEntry, GroundStackFact, ground_stack_matches};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeSelector {
    pub position: Position,
    pub ordinal: u8,
    pub appearance: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    pub position: Option<Position>,
    pub unique: Option<u16>,
    pub action: Option<u16>,
    pub appearance: Option<u16>,
    /// Exact plain-base counterpart; legacy selectors remain an alternative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native: Option<NativeSelector>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Passage {
    pub position: Position,
    pub appearance: u16,
    #[serde(default)]
    pub replacement: Option<u16>,
    /// Source-asymmetric lever: remove `appearance`, then create this different wall.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub create_on_close: Option<u16>,
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
    /// A read-only gate: validate the locked value without resetting later progress.
    #[serde(default, skip_serializing_if = "is_false")]
    pub preserve: bool,
    /// Add `set` to the locked value instead of replacing it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub increment: bool,
    /// Preserve higher progress while raising lower values to `set`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub at_least: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

mod native;
use native::valid_native_callback;

/// One carried item applied to the selected world object.  This deliberately is not a
/// recipe language: the receipt owner locks the source and consumes, changes or retains it.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemOnTarget {
    pub item: String,
    /// Exact interchangeable source definitions, never wildcard identity matching.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<String>,
    pub count: u16,
    pub replacement: Option<String>,
    pub target_replacement: Option<u16>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub reusable: bool,
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
pub struct CallbackEffect {
    pub position: Position,
    pub effect: u16,
}

/// Only value-free map callbacks can be triggered by monsters.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    #[default]
    Player,
    Monster,
}
impl ActorKind {
    fn is_player(&self) -> bool {
        *self == Self::Player
    }
}

/// A static native anchor, optionally guarded by an ordered durable Ground stack.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldRequirement {
    pub position: Position,
    pub appearance: u16,
    /// Exact durable Ground roots, bottom first, in addition to the static anchor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground_stack: Option<Vec<GroundStackEntry>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Callback {
    pub key: String,
    pub quest: String,
    pub trigger: Trigger,
    #[serde(default, skip_serializing_if = "ActorKind::is_player")]
    pub actor_kind: ActorKind,
    pub selector: Selector,
    /// Both lever appearances. None for a step tile. One-way levers refuse their second use.
    pub lever: Option<[u16; 2]>,
    pub reversible: bool,
    /// Durable requirements and inputs apply on opening; closing is a value-free map action.
    #[serde(default, skip_serializing_if = "is_false")]
    pub opening_only: bool,
    /// A personal progress write must carry the registered native world occurrence.
    #[serde(default, skip_serializing_if = "is_false")]
    pub requires_world_admission: bool,
    pub passages: Vec<Passage>,
    pub teleport: Option<Position>,
    /// Source cardinal direction (north/east/south/west = 0/1/2/3) after teleport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facing: Option<u8>,
    #[serde(default, skip_serializing_if = "CallbackEffectSet::is_empty")]
    pub effects: CallbackEffectSet,
    pub storage: Vec<Storage>,
    /// Evaluated with SET_NOW against the fenced writer transaction clock.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_ops: Vec<super::reward_callbacks::StateOperation>,
    /// Opening predicates for value-free map puzzles, evaluated under the Channel turn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub world_requirements: Vec<WorldRequirement>,
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
    pub sources: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    callbacks: Vec<Callback>,
}

/// Shared World prerequisites for exact native source callbacks. Ordinary quest
/// travel and exits remain independent of these event-only predicates.
pub fn world_change_requirement(key: &str) -> Option<(&'static str, Option<u8>)> {
    if super::hive_access::gate(key).is_some() {
        return Some(("hive_born", None));
    }
    match key {
        "oteryn:quest-callback/fire-feathered-native-darkness"
        | "oteryn:quest-callback/fire-feathered-native-life"
        | "oteryn:quest-callback/fire-feathered-native-light" => {
            Some(("the_fire_feathered_serpent", Some(3)))
        }
        "oteryn:quest-callback/grimvale-full-moon-entry" => Some(("grimvale", None)),
        "oteryn:quest-callback/mages-tower-entry" => Some(("the_mage_s_tower", None)),
        "oteryn:quest-callback/deeplings-native-boss-28574"
        | "oteryn:quest-callback/deeplings-native-boss-28575"
        | "oteryn:quest-callback/deeplings-native-boss-28576"
        | "oteryn:quest-callback/deeplings-boss-28574"
        | "oteryn:quest-callback/deeplings-boss-28575"
        | "oteryn:quest-callback/deeplings-boss-28576" => Some(("deeplings", Some(3))),
        _ => None,
    }
}

/// Read the same closed content shape at boot and at the durable mutation boundary.
pub fn load() -> Result<Vec<Callback>, String> {
    let mut callbacks = parse(EMBEDDED)?;
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/world-callbacks/l6-ancient-instruments.json"
    ))?);
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/world-callbacks/l6-item-rewards.json"
    ))?);
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/world-callbacks/l6-tools.json"
    ))?);
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/world-callbacks/l6-world-puzzles.json"
    ))?);
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/world-callbacks/l6-tomb-rooms.json"
    ))?);
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/shared-access-callbacks.json"
    ))?);
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/shared-outfit-callbacks.json"
    ))?);
    callbacks.extend(super::carried_step::callbacks());
    callbacks.extend(super::shared_switches::callbacks());
    if callbacks.len() > MAX_CALLBACKS.saturating_sub(2) {
        return Err("callback bounds including native source callbacks".into());
    }
    callbacks.extend(super::fire_feathered_native::callbacks());
    callbacks.extend(super::shared_world_progress::callbacks());
    callbacks.extend(super::hive_access::callbacks());
    callbacks.push(super::targuna_ram::callback()?);
    callbacks.push(super::rotten_blood::altar_callback());
    callbacks.push(super::schrodinger_island::callback());
    callbacks.push(super::source_world_objects::uneasy_crystal());
    let keys: BTreeSet<_> = callbacks.iter().map(|c| &c.key).collect();
    if keys.len() != callbacks.len() {
        return Err("duplicate callback across registries".into());
    }
    callbacks.extend(super::deepling_boss_access::callbacks(&callbacks)?);
    if callbacks.len() > MAX_CALLBACKS {
        return Err("callback bounds including native access".into());
    }
    counterparts::apply(&mut callbacks)?;
    validate_registry(&callbacks)?;
    callbacks.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(callbacks)
}

fn validate_registry(callbacks: &[Callback]) -> Result<(), String> {
    let mut keys = BTreeSet::new();
    let mut selectors = BTreeSet::new();
    for callback in callbacks {
        if !keys.insert(&callback.key) {
            return Err("duplicate callback across registries".into());
        }
        let selector = serde_json::to_vec(&(callback.trigger, &callback.selector))
            .map_err(|error| error.to_string())?;
        if !selectors.insert(selector) {
            return Err("duplicate callback selector across registries".into());
        }
    }
    Ok(())
}

pub(super) fn parse(text: &str) -> Result<Vec<Callback>, String> {
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
        let operation_tracks: BTreeSet<_> = callback.state_ops.iter().map(|s| &s.track).collect();
        if operation_tracks.len() != callback.state_ops.len()
            || !operation_tracks.is_disjoint(&unique_storage)
            || callback.storage.len() + callback.state_ops.len() > MAX_EFFECTS
            || callback.state_ops.iter().any(|op| !op.valid())
            || (!callback.state_ops.is_empty()
                && (callback.actor_kind != ActorKind::Player
                    || callback.reversible
                    || callback.opening_only
                    || callback.native_rule.is_some()
                    || !callback.world_requirements.is_empty()))
            || !super::valid_quest_key(&callback.key)
            || !super::valid_quest_key(&callback.quest)
            || !keys.insert(&callback.key)
            || (selector.position.is_none()
                && selector.unique.is_none()
                && selector.action.is_none()
                && selector.native.is_none())
            || selector.position.is_some_and(|p| !valid_pos(p))
            || selector
                .native
                .as_ref()
                .is_some_and(|n| !valid_pos(n.position) || n.appearance == 0)
            || callback.teleport.is_some_and(|p| !valid_pos(p))
            || !effects::valid(callback)
            || callback
                .effects
                .teleport_effect
                .is_some_and(|effect| effect == 0 || effect > 255 || callback.teleport.is_none())
            || (callback.actor_kind == ActorKind::Monster
                && (callback.selector.position.is_none()
                    || callback.trigger == Trigger::Use
                    || !!callback.has_personal_state()
                    || !callback.offerings.is_empty()
                    || callback.item_on_target.is_some()
                    || callback.teleport.is_some()
                    || callback.minimum_level.is_some()
                    || callback.experience.is_some()
                    || callback.monk_reward.is_some()
                    || callback.native_rule.is_some()
                    || callback.effects.item_reward.is_some()
                    || callback.effects.outfit_reward.is_some()
                    || callback.effects.mount_reward.is_some()
                    || callback.effects.encounter.is_some()
                    || callback.effects.portals.is_some()
                    || callback.effects.item_cycle.is_some()
                    || !callback.passages.is_empty()))
            || ((callback.requires_world_admission || callback.opening_only)
                && (callback.storage.iter().all(|state| state.preserve)
                    || callback.native_rule.is_some()))
            || (callback.opening_only
                && (callback.trigger != Trigger::Use
                    || callback.lever.is_none()
                    || !callback.reversible
                    || !callback.has_personal_state()
                    || callback.offerings.is_empty()
                    || callback.experience.is_some()
                    || callback.monk_reward.is_some()
                    || callback.item_on_target.is_some()
                    || callback.native_rule.is_some()
                    || callback.effects.item_reward.is_some()
                    || callback.effects.outfit_reward.is_some()
                    || callback.effects.mount_reward.is_some()
                    || callback.teleport.is_some()
                    || callback.effects.encounter.is_some()))
            || callback.world_requirements.len() > MAX_EFFECTS
            || callback.world_requirements.iter().any(|r| {
                !valid_pos(r.position)
                    || r.appearance == 0
                    || r.ground_stack.as_ref().is_some_and(|stack| {
                        callback.actor_kind != ActorKind::Player || !ground_stack::valid(stack)
                    })
            })
            || (!callback.world_requirements.is_empty()
                && ((callback.has_personal_state() && !callback.requires_world_admission)
                    || !callback.offerings.is_empty()
                    || callback.item_on_target.is_some()
                    || callback.minimum_level.is_some()
                    || callback.experience.is_some()
                    || callback.monk_reward.is_some()
                    || callback.native_rule.is_some()
                    || callback.effects.item_reward.is_some()
                    || callback.effects.outfit_reward.is_some()
                    || callback.effects.mount_reward.is_some()))
            || !effects::valid(callback)
            || callback
                .facing
                .is_some_and(|facing| facing > 3 || callback.teleport.is_none())
            || callback.effects.len() > MAX_EFFECTS
            || callback
                .effects
                .iter()
                .any(|e| !valid_pos(e.position) || e.effect == 0)
            || unique_offerings.len() != callback.offerings.len()
            || unique_passages.len() != callback.passages.len()
            || unique_storage.len() != callback.storage.len()
            || callback.passages.len() > MAX_EFFECTS
            || callback.passages.iter().any(|p| {
                p.create_on_close.is_some_and(|id| {
                    id == 0
                        || id == p.appearance
                        || p.replacement.is_some()
                        || callback.passages.len() != 1
                        || callback.lever.is_none()
                        || !callback.reversible
                        || !!callback.has_personal_state()
                        || callback.teleport.is_some()
                        || callback.item_on_target.is_some()
                        || !callback.offerings.is_empty()
                        || !callback.effects.is_empty()
                        || callback.experience.is_some()
                        || callback.monk_reward.is_some()
                        || callback.success_message.is_some()
                })
            })
            || callback.storage.len() > MAX_EFFECTS
            || callback
                .minimum_level
                .is_some_and(|level| level == 0 || !callback.has_personal_state())
            || callback.offerings.len() > MAX_EFFECTS
            || callback
                .experience
                .is_some_and(|xp| !(1..=super::QUESTGATE0_RL_05).contains(&xp))
            || callback.monk_reward.as_ref().is_some_and(|reward| {
                callback.quest != super::monk_shrines::QUEST
                    || callback.trigger != Trigger::Use
                    || !callback.has_personal_state()
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
            || (!callback.offerings.is_empty() && !callback.has_personal_state())
            || (callback.key.starts_with("oteryn:quest-callback/m01b/")
                && !imported_explore::admitted(callback))
            || callback.sources.is_empty()
            || !(imported_explore::admitted(callback)
                || callback.sources.iter().any(|source| {
                    // Dryad Garden uses the pinned Crystal donor; the earlier callbacks use Canary.
                    source.starts_with("canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:")
                        || source.starts_with("crystal:201189f871d23597b82caf88fd611454a2ddefb8:")
                        || (callback.effects.outfit_reward.is_some()
                            && source
                                .starts_with("crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:"))
                        || source
                            .starts_with("crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:")
                }))
            || callback.item_on_target.as_ref().is_some_and(|item| {
                let accepted: BTreeSet<_> = std::iter::once(item.item.as_str())
                    .chain(item.alternatives.iter().map(String::as_str))
                    .collect();
                item.target_replacement == Some(0)
                    || (item.replacement.is_some() && item.count != 1)
                    || !callback.has_personal_state()
                    || callback.trigger != Trigger::Use
                    || item.count == 0
                    || item.count > 100
                    || (item.reusable && (item.count != 1 || item.replacement.is_some()))
                    || !super::valid_quest_key(&item.item)
                    || item.alternatives.len() > 3
                    || accepted.len() != item.alternatives.len() + 1
                    || item
                        .alternatives
                        .iter()
                        .any(|key| !super::valid_quest_key(key))
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
            || callback.storage.iter().any(|s| {
                !super::valid_quest_key(&s.track)
                    || s.minimum > s.maximum
                    || (s.at_least && (s.preserve || s.increment || callback.native_rule.is_some()))
                    || (s.increment
                        && (s.preserve
                            || callback.native_rule.is_some()
                            || s.set <= 0
                            || s.maximum.checked_add(s.set).is_none()))
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
    for callback in callbacks.iter().filter(|c| c.has_personal_state()) {
        for storage in &callback.storage {
            let maximum = if storage.increment {
                storage
                    .maximum
                    .checked_add(storage.set)
                    .ok_or("callback increment overflow")?
            } else {
                storage.set.max(storage.maximum)
            };
            if let Some(track) = tracks.iter_mut().find(|track| track.key == storage.track) {
                if callback.native_rule.is_none() && !storage.preserve {
                    track.max = track.max.max(maximum);
                }
            } else {
                if callback.native_rule.is_some() {
                    return Err(format!(
                        "native callback {} requires an existing track {}",
                        callback.key, storage.track
                    ));
                }
                let initial = imported_explore::initial(callback, storage);
                tracks.push(QuestTrack {
                    key: storage.track.clone(),
                    quest: callback.quest.clone(),
                    initial,
                    min: initial,
                    max: if storage.preserve {
                        storage.maximum
                    } else {
                        maximum
                    },
                });
            }
        }
        for operation in &callback.state_ops {
            if let Some(track) = tracks.iter().find(|track| track.key == operation.track) {
                if track.quest != callback.quest {
                    return Err(format!("foreign world clock track {}", operation.track));
                }
            } else {
                tracks.push(QuestTrack {
                    key: operation.track.clone(),
                    quest: callback.quest.clone(),
                    initial: -1,
                    min: -1,
                    max: i64::MAX,
                });
            }
        }
        transitions.push(primary_transition(callback)?);
        if let Some(completion) = completion_transition(callback) {
            transitions.push(completion);
        }
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

/// Exact canonical state operation used by both lowering and the mutation boundary.
pub fn primary_transition(callback: &Callback) -> Result<QuestTransition, String> {
    let binding = gated_binding(callback)?;
    let bound_index = callback.storage.iter().position(|state| !state.preserve);
    Ok(QuestTransition {
        world_change: callback.world_change.clone(),
        key: callback.key.clone(),
        quest: callback.quest.clone(),
        effects: callback
            .storage
            .iter()
            .enumerate()
            .map(|(index, state)| QuestEffect {
                track: state.track.clone(),
                from: QuestComparison::Between(state.minimum, state.maximum),
                effect: match callback.native_rule {
                    Some(rule) => QuestEffectKind::Native(rule),
                    None if state.preserve => QuestEffectKind::Add(0),
                    None if state.at_least => {
                        match binding.filter(|_| Some(index) == bound_index) {
                            Some(binding) => QuestEffectKind::RaiseToWithBinding {
                                value: state.set,
                                binding,
                            },
                            None => QuestEffectKind::RaiseTo(state.set),
                        }
                    }
                    None if state.increment => match binding.filter(|_| Some(index) == bound_index)
                    {
                        Some(binding) => QuestEffectKind::AddWithBinding {
                            delta: state.set,
                            binding,
                        },
                        None => QuestEffectKind::Add(state.set),
                    },
                    None => match binding.filter(|_| Some(index) == bound_index) {
                        Some(binding) => QuestEffectKind::SetWithBinding {
                            value: state.set,
                            binding,
                        },
                        None => QuestEffectKind::Set(state.set),
                    },
                },
            })
            .chain(
                callback
                    .state_ops
                    .iter()
                    .map(|operation| operation.quest_effect()),
            )
            .collect(),
        completes: callback.effects.completes_quest,
        experience: callback.experience,
    })
}

/// New physical bindings omit display/citation fields and leave every legacy hash unchanged.
pub fn gated_binding(callback: &Callback) -> Result<Option<[u8; 32]>, String> {
    if !callback.requires_world_admission && !callback.opening_only {
        return Ok(None);
    }
    use sha2::{Digest, Sha256};
    let mut value = serde_json::to_value(callback).map_err(|error| error.to_string())?;
    let fields = value.as_object_mut().ok_or("world callback object")?;
    fields.remove("sources");
    fields.remove("success_message");
    fields.remove("failure_message");
    let bytes = serde_json::to_vec(&value).map_err(|error| error.to_string())?;
    Ok(Some(Sha256::digest(bytes).into()))
}

#[cfg(test)]
mod timed_tests;

#[cfg(test)]
mod clock_tests;

#[cfg(test)]
mod literal_text_tests;

#[cfg(test)]
mod cycle_tests;

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
    fn native_counterparts_and_teleport_cues_fail_closed() -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|c| c.key.ends_with("/ancient-tombs-morguthis-entry"))
            .ok_or("native room")?;
        let wire = |c: &Callback| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
        parse(&wire(&callback))?;
        let mut invalid = callback.clone();
        invalid
            .selector
            .native
            .as_mut()
            .ok_or("native binding")?
            .position
            .floor = 1;
        assert!(parse(&wire(&invalid)).is_err());
        let mut invalid = callback.clone();
        invalid
            .selector
            .native
            .as_mut()
            .ok_or("native binding")?
            .appearance = 0;
        assert!(parse(&wire(&invalid)).is_err());
        for effect in [0, 256] {
            let mut invalid = callback.clone();
            invalid.effects.teleport_effect = Some(effect);
            assert!(parse(&wire(&invalid)).is_err());
        }
        let mut invalid = callback.clone();
        invalid.teleport = None;
        assert!(parse(&wire(&invalid)).is_err());
        let mut native_only = callback;
        native_only.selector.unique = None;
        parse(&wire(&native_only))?;
        native_only.selector.native = None;
        assert!(parse(&wire(&native_only)).is_err(), "no wildcard selector");
        let mut closed_rule = load()?
            .into_iter()
            .find(|c| c.native_rule.is_some())
            .ok_or("closed source rule")?;
        parse(&wire(&closed_rule))?;
        closed_rule.selector.native = Some(NativeSelector {
            position: Position {
                x: 7,
                y: 1,
                floor: -7,
            },
            ordinal: 0,
            appearance: 415,
        });
        assert!(
            parse(&wire(&closed_rule)).is_err(),
            "closed source rule cannot acquire an unrelated alias"
        );
        Ok(())
    }

    #[test]
    fn monster_puzzles_and_world_predicates_reject_value_participants() -> Result<(), String> {
        let mut callback = load()?
            .into_iter()
            .find(|c| c.key.ends_with("/behemoth"))
            .ok_or("template")?;
        callback.actor_kind = ActorKind::Monster;
        callback.trigger = Trigger::StepIn;
        callback.selector.position = Some(Position {
            x: 32478,
            y: 31904,
            floor: -5,
        });
        callback.selector.unique = None;
        callback.selector.action = None;
        callback.selector.appearance = Some(431);
        callback.lever = None;
        callback.reversible = false;
        callback.storage.clear();
        callback.passages.clear();
        callback.offerings.clear();
        callback.teleport = None;
        callback.effects.transform = Some(430);
        callback.effects.item_presence = Some(ItemPresence {
            position: Position {
                x: 32478,
                y: 31904,
                floor: -5,
            },
            appearance: 1948,
        });
        let wire = |c: &Callback| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
        parse(&wire(&callback))?;
        for field in [
            "storage",
            "experience",
            "teleport",
            "minimum_level",
            "item_on_target",
        ] {
            let mut value = serde_json::to_value(&callback).map_err(|e| e.to_string())?;
            value[field] = match field {
                "storage" => {
                    serde_json::json!([{"track":"oteryn:quest-progress/fixture/test","minimum":-1,"maximum":0,"set":1}])
                }
                "experience" => 1.into(),
                "minimum_level" => 30.into(),
                "teleport" => serde_json::json!({"x":1,"y":1,"floor":-7}),
                _ => {
                    serde_json::json!({"item":"oteryn:item.tibia.i3031","count":1,"replacement":null,"target_replacement":null})
                }
            };
            assert!(
                parse(&serde_json::json!({"schema":1,"callbacks":[value]}).to_string()).is_err(),
                "monster accepted {field}"
            );
        }
        callback.actor_kind = ActorKind::Player;
        callback.world_requirements = vec![WorldRequirement {
            position: Position {
                x: 1,
                y: 1,
                floor: -7,
            },
            appearance: 2773,
            ground_stack: None,
        }];
        parse(&wire(&callback))?;
        callback.experience = Some(1);
        assert!(parse(&wire(&callback)).is_err());
        Ok(())
    }

    #[test]
    fn registry_refuses_different_keys_on_the_same_selector() -> Result<(), String> {
        let mut callbacks = load()?;
        let mut alias = callbacks
            .iter()
            .find(|callback| callback.key.ends_with("triangle-tower-lever"))
            .ok_or("triangle callback")?
            .clone();
        alias.key = "oteryn:quest-callback/triangle-selector-alias".into();
        callbacks.push(alias);
        assert_eq!(
            validate_registry(&callbacks),
            Err("duplicate callback selector across registries".into())
        );
        Ok(())
    }

    #[test]
    fn effect_wire_forms_preserve_receipt_serialization_and_reject_mixing() -> Result<(), String> {
        for wire in [
            serde_json::json!([{"position": {"x": 32968, "y": 32630, "floor": -7}, "effect": 54}]),
            serde_json::json!({"transform": null, "facing": "south", "magic": null,
                "portals": null, "item_cycle": null, "encounter": null}),
        ] {
            let effects: CallbackEffectSet =
                serde_json::from_value(wire.clone()).map_err(|e| e.to_string())?;
            assert_eq!(
                serde_json::to_value(&effects).map_err(|e| e.to_string())?,
                wire
            );
        }
        let mut mixed: CallbackEffectSet = serde_json::from_value(serde_json::json!([
            {"position": {"x": 32968, "y": 32630, "floor": -7}, "effect": 54}
        ]))
        .map_err(|e| e.to_string())?;
        mixed.facing = Some(Facing::South);
        assert!(serde_json::to_value(&mixed).is_err());
        assert!(
            serde_json::from_value::<CallbackEffectSet>(serde_json::json!({"unknown": true}))
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn qengine_shared_access_rejects_ambiguous_or_unbounded_effects_questpc4() -> Result<(), String>
    {
        let mut source: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/shared-access-callbacks.json"
        ))
        .map_err(|e| e.to_string())?;
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/content-schema/quest-authoring/samples/qengine-access-test-fixtures.json"
        ))).map_err(|error| error.to_string())?;
        // The committed canonical row now owns Triangle. Supplement only missing
        // fixture keys, retaining the actual production callback under test.
        let fixture_callbacks = fixtures["callbacks"].as_array().ok_or("fixtures")?;
        let rows = source["callbacks"].as_array_mut().ok_or("callbacks")?;
        for fixture in fixture_callbacks {
            if !rows.iter().any(|row| row["key"] == fixture["key"]) {
                rows.push(fixture.clone());
            }
        }
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
    fn reusable_tools_require_one_unchanged_item_and_bounded_increments() -> Result<(), String> {
        let source: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/world-callbacks/l6-tools.json"
        ))
        .map_err(|error| error.to_string())?;
        assert!(parse(&source.to_string()).is_ok());
        for count in [0, 2] {
            let mut bad = source.clone();
            bad["callbacks"][0]["item_on_target"]["count"] = count.into();
            assert!(parse(&bad.to_string()).is_err());
        }
        let mut bad = source.clone();
        bad["callbacks"][0]["item_on_target"]["replacement"] = "oteryn:item.tibia.i3304".into();
        assert!(parse(&bad.to_string()).is_err());
        for alternatives in [
            serde_json::json!(["oteryn:item.tibia.i3304"]),
            serde_json::json!(["oteryn:item.tibia.i9598", "oteryn:item.tibia.i9598"]),
            serde_json::json!([""]),
            serde_json::json!([
                "oteryn:item.a",
                "oteryn:item.b",
                "oteryn:item.c",
                "oteryn:item.d"
            ]),
        ] {
            let mut bad = source.clone();
            bad["callbacks"][0]["item_on_target"]["alternatives"] = alternatives;
            assert!(parse(&bad.to_string()).is_err());
        }
        for (field, value) in [
            ("preserve", serde_json::json!(true)),
            ("maximum", serde_json::json!(i64::MAX)),
            ("set", serde_json::json!(0)),
        ] {
            let mut bad = source.clone();
            bad["callbacks"][0]["storage"][1][field] = value;
            assert!(parse(&bad.to_string()).is_err(), "{field}");
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
            assert!(!callback.has_personal_state());
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

#[cfg(test)]
mod worldchg_grimvale_tests {
    #![allow(
        clippy::expect_used,
        reason = "canonical native source fixtures must exist"
    )]
    use super::*;

    #[test]
    fn worldchg_mage_native_entry_and_escape_are_stateless() {
        let callbacks = load().expect("canonical callbacks");
        let entry = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-callback/mages-tower-entry")
            .expect("native Mage entry callback");
        let exit = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-callback/mages-tower-exit")
            .expect("native Mage escape callback");
        assert_eq!(
            entry.selector.position,
            Some(Position {
                x: 33143,
                y: 31527,
                floor: -2
            })
        );
        assert_eq!(entry.selector.appearance, Some(10840));
        assert_eq!(
            entry.selector.unique, None,
            "startup UID is not retained B3 metadata"
        );
        assert_eq!(
            entry.teleport,
            Some(Position {
                x: 32942,
                y: 31594,
                floor: -8
            })
        );
        assert_eq!(
            world_change_requirement(&entry.key),
            Some(("the_mage_s_tower", None))
        );
        assert_eq!(
            exit.selector.position,
            Some(Position {
                x: 32942,
                y: 31592,
                floor: -8
            })
        );
        assert_eq!(exit.selector.appearance, Some(10842));
        assert_eq!(
            exit.teleport,
            Some(Position {
                x: 33143,
                y: 31528,
                floor: -2
            })
        );
        assert_eq!(
            world_change_requirement(&exit.key),
            None,
            "escape always remains available"
        );
        assert!(entry.storage.is_empty() && exit.storage.is_empty());
    }

    #[test]
    fn worldchg_grimvale_source_entry_is_exact_and_leaves_travel_stateless() -> Result<(), String> {
        let callbacks = load()?;
        let entry = callbacks
            .iter()
            .find(|callback| callback.key == "oteryn:quest-callback/grimvale-full-moon-entry")
            .ok_or("Grimvale's source encounter entry has no native callback")?;
        assert_eq!(entry.trigger, Trigger::StepIn);
        assert_eq!(
            entry.selector.position,
            Some(Position {
                x: 33430,
                y: 31537,
                floor: -11
            })
        );
        assert_eq!(entry.selector.appearance, Some(22761));
        assert_eq!(
            entry.selector.action, None,
            "literal source position binds bootstrap markers absent from B3"
        );
        assert_eq!(
            entry.teleport,
            Some(Position {
                x: 33419,
                y: 31539,
                floor: -10
            })
        );
        assert!(
            entry.storage.is_empty(),
            "the moon gate must not fabricate personal quest progress"
        );
        assert_eq!(
            world_change_requirement(&entry.key),
            Some(("grimvale", None))
        );
        assert!(entry.offerings.is_empty());
        assert!(entry.return_on_failure);
        let ordinary = callbacks
            .iter()
            .filter(|callback| {
                callback.key.starts_with("oteryn:quest-callback/grimvale-")
                    && callback.key != entry.key
            })
            .collect::<Vec<_>>();
        assert_eq!(ordinary.len(), 9);
        assert!(
            ordinary
                .iter()
                .all(|callback| !callback.has_personal_state()
                    && world_change_requirement(&callback.key).is_none())
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "world_callbacks/readable_text_tests.rs"]
mod readable_text_tests;

/// A child completion is usable only as a participant of its native primary callback.
/// The durable owner refuses direct requests for this synthetic transition.
pub fn completion_transition(callback: &Callback) -> Option<QuestTransition> {
    use sha2::{Digest, Sha256};
    callback.effects.quest_completion.as_ref()?;
    // Preserve the published parent definition while the child pins the exact
    // native selector, reward and prerequisites. Human citations are not authority.
    let mut value = serde_json::to_value(callback).ok()?;
    let fields = value.as_object_mut()?;
    for field in ["sources", "success_message", "failure_message"] {
        fields.remove(field);
    }
    let binding: String = Sha256::digest(serde_json::to_vec(&value).ok()?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Some(QuestTransition {
        world_change: None,
        key: format!("oteryn:quest-completion/{binding}"),
        quest: callback.effects.quest_completion.clone()?,
        effects: Vec::new(),
        completes: true,
        experience: None,
    })
}

#[cfg(test)]
mod completion_tests;

#[cfg(test)]
mod srv7_mount_reward_tests {
    #[test]
    fn srv7_world_mount_reward_requires_once_only_progress() -> Result<(), String> {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/shared-outfit-callbacks.json"
        ))
        .map_err(|e| e.to_string())?;
        let callback = &mut value["callbacks"][0];
        callback["effects"]
            .as_object_mut()
            .ok_or("effects")?
            .remove("outfit_reward");
        callback["requires_world_admission"] = true.into();
        callback["effects"]["mount_reward"] =
            serde_json::json!({"mount":"oteryn:content.mount.reliable_ram"});
        callback["sources"] = serde_json::json!([
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/targuna/actions_reliable_ram.lua:20-50"
        ]);
        super::parse(&value.to_string())?;
        for storage in value["callbacks"][0]["storage"]
            .as_array_mut()
            .ok_or("storage")?
        {
            storage["set"] = storage["minimum"].clone();
        }
        assert!(
            super::parse(&value.to_string()).is_err(),
            "repeatable mount grants must fail closed"
        );
        Ok(())
    }
}

#[cfg(test)]
mod m07_trials_tests;

#[cfg(test)]
mod qn1_lifecycle_tests {
    #[test]
    fn uneasy_alliance_crystal_has_fenced_ten_second_reset() -> Result<(), String> {
        let callbacks = super::load()?;
        let callback = callbacks
            .iter()
            .find(|c| c.key.ends_with("uneasy-alliance-crystal"))
            .ok_or("missing native crystal callback")?;
        assert_eq!(callback.selector.action, Some(40030));
        assert_eq!(callback.effects.transform, Some(10187));
        let value = serde_json::to_value(&callback.effects).map_err(|e| e.to_string())?;
        assert_eq!(value["reset_after_ms"], 10000);
        assert_eq!(callback.storage[0].minimum, 3);
        assert_eq!(callback.storage[0].maximum, 3);
        assert_eq!(callback.storage[0].set, 4);
        let lowered =
            crate::quest::loader::load_embedded_quest_state("qn1").map_err(|e| format!("{e:?}"))?;
        let catalogue = lowered.catalogue();
        super::parse(&serde_json::json!({"schema": 1, "callbacks": [callback]}).to_string())?;
        let transition = catalogue.transition(&callback.key).ok_or("transition")?;
        let track = callback.storage[0].track.clone();
        let stored = std::collections::BTreeMap::from([(track.clone(), 3)]);
        let changes = catalogue
            .evaluate(transition, &stored, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!((changes[0].before, changes[0].after), (3, 4));
        for wrong_stage in [0, 2, 4, 5] {
            assert!(
                catalogue
                    .evaluate(
                        transition,
                        &std::collections::BTreeMap::from([(track.clone(), wrong_stage)]),
                        0
                    )
                    .is_err()
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod srv7_mount_reward_replay_tests {
    #[test]
    fn srv7_world_mount_reward_requires_once_only_progress() -> Result<(), String> {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/shared-outfit-callbacks.json"
        ))
        .map_err(|e| e.to_string())?;
        let callback = &mut value["callbacks"][0];
        callback["effects"]
            .as_object_mut()
            .ok_or("effects")?
            .remove("outfit_reward");
        callback["requires_world_admission"] = true.into();
        callback["effects"]["mount_reward"] =
            serde_json::json!({"mount":"oteryn:content.mount.reliable_ram"});
        callback["sources"] = serde_json::json!([
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/targuna/actions_reliable_ram.lua:20-50"
        ]);
        super::parse(&value.to_string())?;
        for storage in value["callbacks"][0]["storage"]
            .as_array_mut()
            .ok_or("storage")?
        {
            storage["set"] = storage["minimum"].clone();
        }
        assert!(
            super::parse(&value.to_string()).is_err(),
            "repeatable mount grants must fail closed"
        );
        Ok(())
    }
}
