//! QUESTCB2: bounded, declarative world callbacks. Coordinates use native negative floors.
pub use super::native_rules::NativeRule;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const EMBEDDED: &str = include_str!("../../../../../content/quests/world-callbacks.json");
// Static callbacks and the dynamic altar share the same boot bound.
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

mod counterparts;
mod effects;
pub use effects::{
    CallbackEffectSet, CallbackEffects, Encounter, EncounterMonster, EncounterPlayer,
    EncounterTiming, Facing, ItemCycle, ItemPresence, MagicEffect, Portal, PortalPair,
};

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
    /// Preserve the track value within a receipt-bearing quest transition.
    #[serde(default, skip_serializing_if = "is_false")]
    pub preserve: bool,
    /// Add `set` to the locked value instead of replacing it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub increment: bool,
    /// Preserve higher progress while raising lower values to `set`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub at_least: bool,
    /// Validate quest progress without writing tracks, state, receipts or revisions.
    #[serde(default, skip_serializing_if = "is_false")]
    pub read_only: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

mod counted_use;
pub use counted_use::CountedUse;
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

/// A read-only exact appearance predicate on a native, immovable map entry.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldRequirement {
    pub position: Position,
    pub appearance: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RemoteTeleport {
    pub position: Position,
    pub appearance: u16,
    pub destination: Position,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OccupancyRequirement {
    pub positions: Vec<Position>,
    pub players_only: bool,
    pub eject: Option<Position>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OccupancyItem {
    pub position: Position,
    pub appearance: u16,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Callback {
    /// Only a validated pure STEP-in/out pair may be derived from current creature presence.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub occupancy_driven: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupancy_requirement: Option<OccupancyRequirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub occupancy_items: Vec<OccupancyItem>,
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
    /// Select an alternate destination from the current composed remote tile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_teleport: Option<RemoteTeleport>,
    pub storage: Vec<Storage>,
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
    /// Shared completion lowering reused from questpc8 d90c1bfa2.
    #[serde(default, skip_serializing_if = "is_false")]
    pub completes: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub premium_required: bool,
    /// Native movement origin, used with the selector's actual landing position.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub movement_origins: Vec<Position>,
    /// Actual personal once-claim receipts required in the fenced transaction.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_claims: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counted_use: Option<CountedUse>,
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

impl Callback {
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
    callbacks.extend(parse_with_provenance(
        include_str!("../../../../../content/quests/authored-world-callbacks.json"),
        true,
    )?);
    callbacks.retain(|callback| super::in_runtime_scope(&callback.quest));
    if callbacks.len() > MAX_CALLBACKS.saturating_sub(2) {
        return Err("callback bounds including native source callbacks".into());
    }
    callbacks.extend(super::fire_feathered_native::callbacks());
    callbacks.extend(super::shared_world_progress::callbacks());
    callbacks.extend(super::hive_access::callbacks());
    callbacks.push(super::rotten_blood::altar_callback());
    callbacks.push(super::schrodinger_island::callback());
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
    validate_occupancy_pairs(&callbacks)?;
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

fn parse(text: &str) -> Result<Vec<Callback>, String> {
    parse_with_provenance(text, false)
}

fn parse_with_provenance(text: &str, authored: bool) -> Result<Vec<Callback>, String> {
    let document: Document = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if document.schema != 1 || document.callbacks.len() > MAX_CALLBACKS {
        return Err("callback bounds".into());
    }
    let mut keys = BTreeSet::new();
    for callback in &document.callbacks {
        if callback.occupancy_driven {
            validate_occupancy_shape(callback)?;
        } else if callback.occupancy_requirement.is_some() || !callback.occupancy_items.is_empty() {
            return Err(format!(
                "occupancy rule on imperative callback {}",
                callback.key
            ));
        }
        let selector = &callback.selector;
        let valid_pos = |p: Position| (-15..=0).contains(&p.floor);
        let unique_offerings: BTreeSet<_> = callback.offerings.iter().map(|o| o.position).collect();
        let unique_passages: BTreeSet<_> = callback.passages.iter().map(|p| p.position).collect();
        let unique_storage: BTreeSet<_> = callback.storage.iter().map(|s| &s.track).collect();
        let unique_claims: BTreeSet<_> = callback.required_claims.iter().collect();
        if !super::valid_quest_key(&callback.key)
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
            || callback.movement_origins.iter().any(|p| !valid_pos(*p)
                || callback.trigger != Trigger::StepIn || callback.selector.position.is_none())
            || callback.movement_origins.len() > MAX_EFFECTS
            || callback.movement_origins.iter().collect::<BTreeSet<_>>().len() != callback.movement_origins.len()
            || callback.required_claims.len() > MAX_EFFECTS
            || unique_claims.len() != callback.required_claims.len()
            || callback.required_claims.iter().any(|key| !key.starts_with("oteryn:reward-claim.") || !super::valid_quest_key(key))
            || ((callback.completes || callback.premium_required || !callback.movement_origins.is_empty() || !callback.required_claims.is_empty())
                && (callback.storage.is_empty() || callback.is_read_only() || callback.occupancy_driven))
            || !effects::valid(callback)
            || callback
                .effects
                .teleport_effect
                .is_some_and(|effect| effect == 0 || effect > 255 || callback.teleport.is_none())
            || (callback.actor_kind == ActorKind::Monster
                && (callback.selector.position.is_none()
                    || callback.trigger == Trigger::Use
                    || !callback.storage.is_empty()
                    || !callback.offerings.is_empty()
                    || callback.item_on_target.is_some()
                    || callback.teleport.is_some()
                    || callback.minimum_level.is_some()
                    || callback.experience.is_some()
                    || callback.monk_reward.is_some()
                    || callback.native_rule.is_some()
                    || callback.effects.item_reward.is_some()
                    || callback.effects.outfit_reward.is_some()
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
                    || callback.storage.is_empty()
                    || callback.offerings.is_empty()
                    || callback.experience.is_some()
                    || callback.monk_reward.is_some()
                    || callback.item_on_target.is_some()
                    || callback.native_rule.is_some()
                    || callback.effects.item_reward.is_some()
                    || callback.effects.outfit_reward.is_some()
                    || callback.teleport.is_some()
                    || callback.effects.encounter.is_some()))
            || callback.world_requirements.len() > MAX_EFFECTS
            || callback
                .world_requirements
                .iter()
                .any(|r| !valid_pos(r.position) || r.appearance == 0)
            || (!callback.world_requirements.is_empty()
                && (!callback.storage.is_empty()
                    || !callback.offerings.is_empty()
                    || callback.item_on_target.is_some()
                    || callback.minimum_level.is_some()
                    || callback.experience.is_some()
                    || callback.monk_reward.is_some()
                    || callback.native_rule.is_some()
                    || callback.effects.item_reward.is_some()
                    || callback.effects.outfit_reward.is_some()))
            || !effects::valid(callback)
            || callback
                .facing
                .is_some_and(|facing| facing > 3 || callback.teleport.is_none())
            || callback.effects.len() > MAX_EFFECTS
            || callback
                .effects
                .iter()
                .any(|e| !valid_pos(e.position) || e.effect == 0)
            || !counted_use::valid(callback)
            || callback.remote_teleport.as_ref().is_some_and(|remote| {
                !valid_pos(remote.position)
                    || !valid_pos(remote.destination)
                    || remote.appearance == 0
                    || callback.trigger != Trigger::StepIn
                    || !callback.is_read_only()
                    || callback.teleport.is_none()
            })
            || unique_offerings.len() != callback.offerings.len()
            || unique_passages.len() != callback.passages.len()
            || unique_storage.len() != callback.storage.len()
            || (callback.storage.iter().any(|storage| storage.read_only)
                && (!callback.is_read_only()
                    || !matches!(callback.trigger, Trigger::Use | Trigger::StepIn)
                    || callback.teleport.is_none()
                    || callback.lever.is_some()
                    || !callback.passages.is_empty()
                    || !callback.offerings.is_empty()
                    || callback.item_on_target.is_some()
                    || callback.minimum_level.is_some()
                    || callback.experience.is_some()
                    || callback.monk_reward.is_some()
                    || callback.native_rule.is_some()
                    || !callback.effects.is_empty()
                    || callback.storage.iter().any(|storage| storage.preserve)))
            || callback.passages.len() > MAX_EFFECTS
            || callback.passages.iter().any(|p| {
                p.create_on_close.is_some_and(|id| {
                    id == 0
                        || id == p.appearance
                        || p.replacement.is_some()
                        || callback.passages.len() != 1
                        || callback.lever.is_none()
                        || !callback.reversible
                        || !callback.storage.is_empty()
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
            || (authored && (!callback.key.starts_with("oteryn:quest-callback/authored-")
                || !callback.sources.iter().any(|source| source.starts_with("oteryn-authored:"))))
            || !callback.sources.iter().any(|source| {
                // Dryad Garden uses the pinned Crystal donor; the earlier callbacks use Canary.
                (authored && source.starts_with("oteryn-authored:"))
                    || source.starts_with("canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:")
                    || source.starts_with("crystal:201189f871d23597b82caf88fd611454a2ddefb8:")
                    || ((callback.effects.outfit_reward.is_some() || callback.occupancy_driven)
                        && source.starts_with("crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:"))
                    || source.starts_with("crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:")
            })
            || callback.item_on_target.as_ref().is_some_and(|item| {
                let accepted: BTreeSet<_> = std::iter::once(item.item.as_str())
                    .chain(item.alternatives.iter().map(String::as_str))
                    .collect();
                item.target_replacement == Some(0)
                    || (item.replacement.is_some() && item.count != 1)
                    || callback.storage.is_empty()
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
                .is_some_and(|p| {
                    p[0] == p[1]
                        || (callback.trigger != Trigger::Use
                            && (!callback.reversible
                                || !callback.storage.is_empty()
                                || callback.teleport.is_some()
                                || !callback.offerings.is_empty()
                                || callback.item_on_target.is_some()))
                })
            // Departure restores Channel-local map facts only. It must never
            // introduce another durable transition for the same movement cause.
            || (callback.trigger == Trigger::StepOut
                && ((!callback.reversible && callback.effects.transform.is_none())
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
    for callback in callbacks.iter().filter(|c| !c.storage.is_empty()) {
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
                if callback.native_rule.is_none() && !storage.preserve && !storage.read_only {
                    track.max = track.max.max(maximum);
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
                        maximum
                    },
                });
            }
        }
        let counted_effect = counted_use::extend(callback, &mut tracks)?;
        // Pure gates must not acquire a generic transition: even ADD(0)
        // creates a Character revision, quest state and receipt.
        if callback.is_read_only() {
            continue;
        }
        // Reuse questpc8's immutable plan witness so admission predicates and
        // completion participate in the existing quest definition hash.
        if callback.completes
            || callback.counted_use.is_some()
            || callback.premium_required
            || callback.effects.item_reward.is_some()
            || (callback.effects.portals.is_some() && !callback.storage.is_empty())
            || !callback.movement_origins.is_empty()
            || !callback.required_claims.is_empty()
        {
            let digest: String =
                Sha256::digest(serde_json::to_vec(&callback).map_err(|e| e.to_string())?)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
            transitions.push(QuestTransition {
                world_change: None,
                key: format!("oteryn:quest-world-plan/{digest}"),
                quest: callback.quest.clone(),
                effects: vec![QuestEffect {
                    track: callback
                        .storage
                        .first()
                        .ok_or("missing value guard")?
                        .track
                        .clone(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Computed,
                }],
                completes: false,
                experience: None,
            });
        }
        let mut primary = primary_transition(callback)?;
        primary.effects.extend(counted_effect);
        transitions.push(primary);
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
            .collect(),
        completes: callback.completes,
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
    fn lane_qengine_shared_access_rejects_ambiguous_or_unbounded_effects() -> Result<(), String> {
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
    fn remote_teleport_is_a_bounded_read_only_step() -> Result<(), String> {
        let source: serde_json::Value = serde_json::from_str(include_str!("../../../../../tools/content-schema/quest-authoring/batches/quests4/formogar_mine_hoist.json")).map_err(|e| e.to_string())?;
        let mut callback = source["callbacks"][0].clone();
        callback["remote_teleport"] = serde_json::json!({
            "position": {"x":32156,"y":31125,"floor":-10}, "appearance":2772,
            "destination": {"x":32157,"y":31125,"floor":-11}
        });
        let document =
            |callback| serde_json::json!({"schema":1,"callbacks":[callback]}).to_string();
        assert!(parse(&document(callback.clone())).is_ok());
        for case in 0..5 {
            let mut invalid = callback.clone();
            match case {
                0 => invalid["remote_teleport"]["appearance"] = 0.into(),
                1 => invalid["remote_teleport"]["position"]["floor"] = (-16).into(),
                2 => invalid["remote_teleport"]["destination"]["floor"] = 1.into(),
                3 => invalid["trigger"] = "use".into(),
                _ => invalid["storage"][0]["read_only"] = false.into(),
            }
            assert!(parse(&document(invalid)).is_err());
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
        for case in 0..5 {
            let mut callback = held["callbacks"][1].clone();
            match case {
                0 => callback["experience"] = 1.into(),
                1 => callback["storage"][0]["preserve"] = true.into(),
                2 => callback["trigger"] = "step_out".into(),
                3 => {
                    callback["effects"] = serde_json::json!({"transform": 431, "facing": null, "magic": null, "portals": null, "item_cycle": null, "encounter": null})
                }
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
    #[test]
    fn lane_qengine_l1_world_outfit_sources_are_narrowly_admitted() -> Result<(), String> {
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

fn validate_occupancy_shape(c: &Callback) -> Result<(), String> {
    let mut effects = c.effects.clone();
    effects.transform = None;
    if !matches!(c.trigger, Trigger::StepIn | Trigger::StepOut)
        || c.selector.position.is_none()
        || c.selector.appearance.is_none()
        || c.effects.transform.is_none()
        || !effects.is_empty()
        || c.lever.is_some()
        || c.reversible
        || !c.storage.is_empty()
        || !c.offerings.is_empty()
        || c.teleport.is_some()
        || c.remote_teleport.is_some()
        || c.item_on_target.is_some()
        || c.minimum_level.is_some()
        || c.experience.is_some()
        || c.monk_reward.is_some()
        || c.native_rule.is_some()
        || c.return_on_failure
        || c.failure_message.is_some()
        || c.success_message.is_some()
        || (c.occupancy_requirement.is_none() && c.passages.iter().any(|p| p.replacement.is_none()))
        || c.occupancy_items.len() > 8
        || c.occupancy_items
            .iter()
            .any(|i| i.appearance == 0 || !(-15..=0).contains(&i.position.floor))
        || c.occupancy_items
            .iter()
            .map(|i| i.position)
            .collect::<BTreeSet<_>>()
            .len()
            != c.occupancy_items.len()
        || c.occupancy_requirement.as_ref().is_some_and(|r| {
            !(1..=8).contains(&r.positions.len())
                || r.positions.iter().copied().collect::<BTreeSet<_>>().len() != r.positions.len()
                || r.positions.iter().any(|p| !(-15..=0).contains(&p.floor))
                || c.selector
                    .position
                    .is_none_or(|p| !r.positions.contains(&p))
                || r.eject.is_some_and(|p| {
                    !(-15..=0).contains(&p.floor)
                        || c.passages.is_empty()
                        || c.passages
                            .iter()
                            .any(|passage| passage.replacement.is_some() || passage.position == p)
                })
        })
    {
        return Err(format!(
            "occupancy callback is not a pure source projection {}",
            c.key
        ));
    }
    Ok(())
}

fn validate_occupancy_pairs(callbacks: &[Callback]) -> Result<(), String> {
    for c in callbacks.iter().filter(|c| c.occupancy_driven) {
        validate_occupancy_shape(c)?;
        let pair: Vec<_> = callbacks
            .iter()
            .filter(|p| p.occupancy_driven && p.selector == c.selector)
            .collect();
        if pair.len() != 2
            || pair[0].quest != pair[1].quest
            || pair[0].trigger == pair[1].trigger
            || pair[0].passages != pair[1].passages
            || pair[0].occupancy_requirement != pair[1].occupancy_requirement
            || pair[0].occupancy_items != pair[1].occupancy_items
            || pair
                .iter()
                .find(|p| p.trigger == Trigger::StepOut)
                .is_none_or(|p| p.effects.transform != p.selector.appearance)
        {
            return Err(format!("unpaired or ambiguous occupancy source {}", c.key));
        }
    }
    Ok(())
}

#[cfg(test)]
mod occupancy_tests {
    use super::*;

    #[test]
    fn occupancy_pairs_are_closed_pure_and_have_one_source_restore() -> Result<(), String> {
        let all = load()?;
        let pair: Vec<_> = all
            .into_iter()
            .filter(|c| c.occupancy_driven && c.key.contains("thais-lighthouse"))
            .collect();
        assert_eq!(pair.len(), 2);
        validate_occupancy_pairs(&pair)?;
        for edit in 0..6 {
            let mut bad = pair.clone();
            match edit {
                0 => {
                    bad.pop();
                }
                1 => bad[0].selector.position = None,
                2 => bad[0].minimum_level = Some(1),
                3 => bad[0].passages[0].replacement = None,
                4 => {
                    let out = bad
                        .iter_mut()
                        .find(|c| c.trigger == Trigger::StepOut)
                        .ok_or("out")?;
                    out.effects.transform = Some(999);
                }
                _ => {
                    let mut duplicate = bad[0].clone();
                    duplicate.key.push_str("-duplicate");
                    bad.push(duplicate);
                }
            }
            assert!(validate_occupancy_pairs(&bad).is_err(), "edit {edit}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod grouped_occupancy_tests {
    use super::*;
    #[test]
    fn grouped_projection_rejects_duplicate_unbound_and_invalid_ejection_inputs()
    -> Result<(), String> {
        let all = load()?;
        let pair: Vec<_> = all
            .into_iter()
            .filter(|c| {
                c.key == "oteryn:quest-callback/demon-helmet-pressure-1"
                    || c.key == "oteryn:quest-callback/demon-helmet-pressure-1-out"
            })
            .collect();
        assert_eq!(pair.len(), 2);
        validate_occupancy_pairs(&pair)?;
        for edit in 0..6 {
            let mut bad = pair.clone();
            for c in &mut bad {
                let r = c.occupancy_requirement.as_mut().ok_or("group")?;
                match edit {
                    0 => r.positions.push(r.positions[0]),
                    1 => r.positions.clear(),
                    2 => r.positions[0].floor = -16,
                    3 => r.eject = Some(c.passages[0].position),
                    4 => {
                        r.positions = vec![Position {
                            x: 1,
                            y: 1,
                            floor: -13,
                        }]
                    }
                    _ => c.minimum_level = Some(1),
                }
            }
            assert!(
                validate_occupancy_pairs(&bad).is_err(),
                "malformed group {edit}"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod authored_completion_tests {
    use super::*;
    #[test]
    fn authored_completion_origin_and_claims_are_closed_and_legacy_defaults_are_omitted()
    -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/authored-legion-cave-return")
            .ok_or("return")?;
        let row = serde_json::to_value(callback).map_err(|e| e.to_string())?;
        let document = |row| serde_json::json!({"schema":1,"callbacks":[row]}).to_string();
        assert_eq!(
            parse_with_provenance(&document(row.clone()), true)?.len(),
            1
        );
        assert!(
            parse(&document(row.clone())).is_err(),
            "authored rules require their explicit registry"
        );
        let mut bad = row.clone();
        bad["movement_origins"][0]["floor"] = 1.into();
        assert!(parse_with_provenance(&document(bad), true).is_err());
        let mut bad = row.clone();
        bad["required_claims"] = serde_json::json!([
            "oteryn:reward-claim.kv.legiondawnport",
            "oteryn:reward-claim.kv.legiondawnport"
        ]);
        assert!(parse_with_provenance(&document(bad), true).is_err());
        let mut bad = row.clone();
        bad["required_claims"] = serde_json::json!(["canary:foreign"]);
        assert!(parse_with_provenance(&document(bad), true).is_err());
        let mut bad = row;
        bad["storage"] = serde_json::json!([]);
        assert!(parse_with_provenance(&document(bad), true).is_err());
        for legacy in parse(EMBEDDED)? {
            let row = serde_json::to_value(legacy).map_err(|e| e.to_string())?;
            for key in ["completes", "movement_origins", "required_claims"] {
                assert!(row.get(key).is_none(), "legacy plan gained {key}");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod world_item_reward_tests {
    use super::*;
    #[test]
    fn world_item_rewards_require_bounded_use_and_a_real_progress_advance() -> Result<(), String> {
        let row = load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/authored-nimmersatt-snowbank")
            .ok_or("snowbank")?;
        let row = serde_json::to_value(row).map_err(|e| e.to_string())?;
        let document = |row| serde_json::json!({"schema":1,"callbacks":[row]}).to_string();
        assert_eq!(
            parse_with_provenance(&document(row.clone()), true)?.len(),
            1
        );
        for count in [0, 101] {
            let mut bad = row.clone();
            bad["effects"]["item_reward"]["count"] = count.into();
            assert!(parse_with_provenance(&document(bad), true).is_err());
        }
        let mut bad = row.clone();
        bad["trigger"] = "step_in".into();
        assert!(parse_with_provenance(&document(bad), true).is_err());
        let mut bad = row.clone();
        bad["storage"] = serde_json::json!([]);
        assert!(parse_with_provenance(&document(bad), true).is_err());
        let mut bad = row;
        bad["effects"]["item_reward"]["item"] = "canary:item.fake".into();
        assert!(parse_with_provenance(&document(bad), true).is_err());
        for legacy in parse(EMBEDDED)? {
            let encoded = serde_json::to_value(legacy).map_err(|e| e.to_string())?;
            assert!(encoded.get("premium_required").is_none());
            assert!(encoded["effects"].get("item_reward").is_none());
        }
        Ok(())
    }
}

pub(super) fn validate_counted_use_closed(catalogue: &QuestStateCatalogue) -> Result<(), String> {
    counted_use::validate_closed(catalogue, &load()?)
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
        assert!(ordinary.iter().all(|callback| callback.storage.is_empty()
            && world_change_requirement(&callback.key).is_none()));
        Ok(())
    }
}
