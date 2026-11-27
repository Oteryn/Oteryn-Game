//! QUESTCB2: content callbacks on literal USE and committed step ingress.
use super::{
    ComposedFreshAdmission,
    actor_spell::ChannelSpellStates,
    connection::{UseCommand, UseOutcome},
};
mod admission;
mod amphoras;
mod conditional_cells;
use admission::{character_fence_required, stateless_observation_required};
mod clear_area;
mod committed_copy;
mod court_warlock;
#[cfg(test)]
pub(super) use court_warlock::PRE_SOURCE_BOSS_CHARGE;
#[path = "daily_world_gates.rs"]
mod daily_world_gates;
mod definition_target;
mod dreamer_carrot;
mod effects;
mod encounter;
pub(super) use encounter::room_contains;
mod execute;
mod goblin_elevator;
mod ground_stack;
mod mead;
pub(super) mod monsters;
mod paid_projection;
mod party_transfer;
mod personal_encounter;
mod personal_trial;
mod phase;
mod rafzane_elevator;
mod reward_door;
mod rewards;
mod rotten_flame;
mod scene;
mod scene_actors;
mod scene_items;
mod serpentine_lever;
mod shared_gate;
mod soul_war_reward;
mod source_transforms;
mod timed_encounter;
mod timed_passages;

use super::paced_movement::underwater_drown::underwater;
use crate::durability::{
    character_progression::CurrentCharacterGameplayFence,
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
    quest_world_callback::{QuestWorldContext, may_debit, paid_projection::PaidArrival},
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
    MovementPositionSnapshot,
};
use crate::map::{
    boot::BundleMap,
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_quests::quest::world_callbacks::{
    self, Callback, Position, Trigger, VampireCoffinKind,
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::OnceLock;

type CallbackGuards<'a> = (
    tokio::sync::MutexGuard<'a, ChannelRuntimeV1>,
    tokio::sync::MutexGuard<'a, ChannelSpellStates>,
);

#[derive(Clone)]
struct Pending {
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    callback: String,
    key: u64,
    revision: u64,
    cause: QuestCause,
    origin: Option<MovementPositionSnapshot>,
    context: Option<QuestWorldContext>,
    paid_arrivals: Vec<PaidArrival>,
    flame: Option<crate::durability::quest_state::rotten_flame::RottenFlameUse>,
    ingress_settled: bool,
    participants: Vec<GameSessionId>,
    projection_target: Option<u64>,
    flame_effect_sent: bool,
    /// The exact carried tool bound into the durable World context, if any.
    selected_tool: Option<([u8; 16], Vec<u8>)>,
}

struct ReadOnlyGate {
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    callback: String,
}

/// A World observation is retained only until the native actor/object CAS. It is
/// neither a persisted permission nor a movement grant.
struct ObservedWorldCallback {
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    key: u64,
    revision: u64,
    callback: String,
    context: QuestWorldContext,
}

#[derive(Default)]
pub(super) struct PendingCallbacks {
    pub(super) elemental: std::sync::Mutex<std::collections::HashMap<[u8; 16], ExactActorRef>>,
    personal: personal_encounter::Cohorts,
    scene_item_receipts: scene_items::SceneItemReceipts,
    #[cfg(test)]
    pub(super) delay_scene_commit_ack: std::sync::atomic::AtomicBool,
    #[cfg(test)]
    pub(super) lose_coffin_delivery_ack: std::sync::atomic::AtomicBool,
    #[cfg(test)]
    pub(super) lose_source_boss_commit_ack: std::sync::atomic::AtomicBool,
    #[cfg(test)]
    pub(super) interrupt_source_boss_before_projection: std::sync::atomic::AtomicBool,
    timed: std::sync::Mutex<Vec<timed_encounter::TimedEncounter>>,
    encounters: std::sync::Mutex<Vec<encounter::EncounterVisit>>,
    ground_use_cleanup: std::sync::Mutex<Vec<definition_target::GroundUseTurn>>,
    ground_uses: std::sync::Mutex<
        std::collections::HashMap<GameSessionId, definition_target::PendingGroundUse>,
    >,
    #[cfg(any(test, feature = "test-support"))]
    pub(super) interrupt_ground_use_after_commit: std::sync::atomic::AtomicBool,
    records: std::sync::Mutex<std::collections::HashMap<GameSessionId, Pending>>,
    closing: std::sync::Mutex<std::collections::HashSet<GameSessionId>>,
    carrot_refusal:
        std::sync::Mutex<Option<(ExactActorRef, GameSessionId, MovementPositionSnapshot)>>,
    soul_rewards:
        std::sync::Mutex<std::collections::HashMap<GameSessionId, soul_war_reward::Pending>>,
    #[cfg(test)]
    pub(super) fixture: OnceLock<Vec<Callback>>,
    #[cfg(test)]
    pub(super) reward_claims_fixture: OnceLock<Vec<String>>,
    #[cfg(test)]
    pub(super) interrupt_after_commit: std::sync::atomic::AtomicBool,
    #[cfg(test)]
    pub(super) phase_read_unavailable: std::sync::atomic::AtomicBool,
    #[cfg(test)]
    pub(super) before_commit:
        std::sync::Mutex<Option<std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>>>,
    #[cfg(test)]
    pub(super) observed_corpse_expiry:
        std::sync::Mutex<Option<tokio::sync::oneshot::Sender<tokio::time::Instant>>>,
    #[cfg(test)]
    pub(super) shared_switch_occurred_test: std::sync::Mutex<Option<i64>>,
}

pub(super) struct PaidCallbackClose<'a> {
    callbacks: &'a PendingCallbacks,
    session: GameSessionId,
}

impl Drop for PaidCallbackClose<'_> {
    fn drop(&mut self) {
        if let Ok(mut closing) = self.callbacks.closing.lock() {
            closing.remove(&self.session);
        }
    }
}

impl PendingCallbacks {
    #[cfg(test)]
    pub(super) fn discard_session_projection(&self, session: GameSessionId) -> bool {
        self.records
            .lock()
            .is_ok_and(|mut records| records.remove(&session).is_some())
    }

    pub(super) fn begin_paid_callback_close(
        &self,
        session: GameSessionId,
    ) -> Option<PaidCallbackClose<'_>> {
        if !self.closing.lock().ok()?.insert(session) {
            return None;
        }
        Some(PaidCallbackClose {
            callbacks: self,
            session,
        })
    }

    fn closing(&self, session: GameSessionId) -> bool {
        self.closing
            .lock()
            .map_or(true, |closing| closing.contains(&session))
    }

    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut records) = self.records.lock() {
            records.remove(&session);
        }
    }

    pub(super) fn complete_native_quest(
        &self,
        runtime: &mut ChannelRuntimeV1,
        receipt: &crate::durability::quest_state::CommittedQuestTransition,
        session: GameSessionId,
        observed_revision: u64,
    ) -> Result<(), UseDisposition> {
        if !receipt.completes {
            return Ok(());
        }
        if !matches!(receipt.cause, QuestCause::CreatureDeath(_))
            || observed_revision > receipt.original_character_revision.get()
        {
            return Err(UseDisposition::Rejected);
        }
        self.personal
            .complete(
                runtime,
                receipt.character_id.as_bytes(),
                session,
                &receipt.quest_key,
                observed_revision,
            )
            .map(|_| ())
    }
}

impl PendingCallbacks {
    pub(super) fn next_encounter_due_us(&self) -> Option<u64> {
        self.timed
            .lock()
            .ok()?
            .iter()
            .map(|pending| pending.next_due_us())
            .min()
    }
}

fn changes_carried_items(callback: &Callback) -> bool {
    callback
        .item_on_target
        .as_ref()
        .is_some_and(|tool| !tool.reusable || !tool.ingredients.is_empty())
        || !callback.offerings.is_empty()
        || callback.item_reward().is_some()
        || callback
            .monk_reward
            .as_ref()
            .is_some_and(|reward| reward.item_source_id.is_some())
}

fn callbacks() -> &'static [Callback] {
    static DATA: OnceLock<Vec<Callback>> = OnceLock::new();
    DATA.get_or_init(|| world_callbacks::load().unwrap_or_default())
}
// A shared native station may author disjoint stages. The session copy selects
// a candidate only; the durable Quest owner rechecks every gate before effects.
const EROTH_AUTHORED_MIRROR: &str = "oteryn:quest-callback/to-blind-the-enemy/mirror";
const EROTH_LEGACY_MIRROR: &str = "oteryn:quest-callback/elvenbane-mirror-crystal";

fn unique_eligible<'a>(
    candidates: impl IntoIterator<Item = &'a Callback>,
    tracks: &std::collections::BTreeMap<String, i64>,
) -> Option<&'a Callback> {
    let mut candidates = candidates.into_iter().peekable();
    let first = candidates.next()?;
    let second = candidates.next();
    let passes_storage = |callback: &&Callback| {
        callback.storage.iter().all(|storage| {
            (storage.minimum..=storage.maximum)
                .contains(&tracks.get(&storage.track).copied().unwrap_or(-1))
        })
    };
    if let Some(second) = second
        && candidates.peek().is_none()
        && let Some((authored, supplemental)) = ape_city_route_pair(first, second)
    {
        // The saved route owns the placement even when its use gate is closed.
        // Falling through then could grant the other route's reward or bypass its switches.
        let stage = tracks
            .get("oteryn:quest-progress/chosen/ape-city/stage")
            .copied()
            .unwrap_or_default();
        let selected = if stage > 0 { authored } else { supplemental };
        return passes_storage(&selected).then_some(selected);
    }
    let mut eligible = std::iter::once(first)
        .chain(second)
        .chain(candidates)
        .filter(passes_storage);
    let selected = eligible.next()?;
    let Some(other) = eligible.next() else {
        return Some(selected);
    };
    if eligible.next().is_some() {
        return None;
    }
    // Each pair shares a source object. Use the authored route when both stages
    // are eligible, matching the NPC owners' choice for an active authored journey.
    match (selected.key.as_str(), other.key.as_str()) {
        (
            "oteryn:quest-callback/paradox-skulls",
            "oteryn:quest-callback/paradox-tower/native-skulls",
        )
        | (EROTH_LEGACY_MIRROR, EROTH_AUTHORED_MIRROR) => Some(other),
        (
            "oteryn:quest-callback/paradox-tower/native-skulls",
            "oteryn:quest-callback/paradox-skulls",
        )
        | (EROTH_AUTHORED_MIRROR, EROTH_LEGACY_MIRROR) => Some(selected),
        _ => None,
    }
}

fn ape_city_route_pair<'a>(
    first: &'a Callback,
    second: &'a Callback,
) -> Option<(&'a Callback, &'a Callback)> {
    const AUTHORED: &str = "oteryn:quest-callback/authored-ape-city-";
    const SUPPLEMENTAL: &str = "oteryn:quest-callback/ape-city-";
    let (authored, supplemental) = if first.key.starts_with(AUTHORED) {
        (first, second)
    } else {
        (second, first)
    };
    let authored_source = authored.key.strip_prefix(AUTHORED)?;
    let supplemental_source = supplemental.key.strip_prefix(SUPPLEMENTAL)?;
    let counterpart = match authored_source {
        "deepest-catacombs" => "catacombs-amphora-gate",
        "gate" => "wall-1040",
        "exit-gate" => "wall-1041",
        "holy-hair" | "lever-1" | "lever-2" | "lever-3" | "lever-4" | "lever-5" | "lever-6" => {
            authored_source
        }
        _ => return None,
    };
    (supplemental_source == counterpart).then_some((authored, supplemental))
}

// Retain published keys for receipt reconciliation. Fresh USE has one owner
// for each of these chests when both source programmes are loaded.
fn superseded_chest(callback: &Callback, callbacks: &[Callback]) -> bool {
    let owner = match callback.key.as_str() {
        "oteryn:quest-callback/outlaw-camp-treasure" => {
            "oteryn:quest-callback/authored-outlaw-camp-s3-treasure"
        }
        "oteryn:quest-callback/paradox-tower/talons" => "oteryn:quest-callback/paradox-claim-talon",
        _ => return false,
    };
    callbacks.iter().any(|candidate| candidate.key == owner)
}

fn tile(p: Position) -> TilePos {
    TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}
fn local(p: Position) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(p.x),
        y: i32::from(p.y),
        floor: -i16::from(p.floor),
    }
}
fn native(p: MovementLocalPosition) -> Option<TilePos> {
    Some(TilePos {
        x: p.x.try_into().ok()?,
        y: p.y.try_into().ok()?,
        floor: i8::try_from(p.floor).ok()?.checked_neg()?,
    })
}
fn captured_merchant(map: &BundleMap, key: u64) -> Option<(TilePos, crate::map::view::EntryFacts)> {
    use crate::map::view::{EntrySource, MapFacts};
    if key >> 62 != 2 {
        return None;
    }
    let (pos, id) = map.facts.movement_item(key)?;
    let entry = map.facts.movement_items(pos).into_iter().find(|entry| {
        entry.source == EntrySource::Added(id) && entry.facts.appearance_id == 12189
    })?;
    (!entry.facts.pickupable && !map.facts.house_tile(pos)).then_some((pos, entry.facts))
}

/// Resolve only a closed authored shared callback's actual server-created object.
/// The stable origin and current custody must agree; a dropped lookalike fails.
fn shared_added_source(
    map: &BundleMap,
    callback: &Callback,
    key: u64,
) -> Option<(crate::map::overlay::TilePos, crate::map::view::EntryFacts)> {
    use crate::map::view::{EntrySource, MapFacts};
    if callback.effects.loot_merchant {
        let (pos, facts) = captured_merchant(map, key)?;
        return callback
            .selector
            .matches_position(Position {
                x: pos.x,
                y: pos.y,
                floor: pos.floor,
            })
            .then_some((pos, facts));
    }
    let (appearance, x, y, z) =
        oteryn_gameplay_items::event_planting::world_change_item_source(key)?;
    let pos = crate::map::overlay::TilePos {
        x,
        y,
        floor: -(i8::try_from(z).ok()?),
    };
    if callback.selector.position.map(tile) != Some(pos)
        || callback.selector.appearance != Some(appearance)
        || callback.selector.unique.is_some()
        || callback.selector.action.is_some()
        || !(oteryn_gameplay_quests::quest::shared_world_progress::callbacks()
            .iter()
            .any(|expected| expected == callback)
            || oteryn_gameplay_quests::quest::fire_feathered_native::callbacks()
                .iter()
                .any(|expected| expected == callback)
            || oteryn_gameplay_quests::quest::deepling_boss_access::matches(
                &callback.key,
                Position {
                    x: pos.x,
                    y: pos.y,
                    floor: pos.floor,
                },
                appearance,
            ))
        || map.facts.house_tile(pos)
        || map.facts.movement_item(key)?.0 != pos
    {
        return None;
    }
    let entry = map
        .facts
        .movement_items(pos)
        .into_iter()
        .find(|entry| matches!(entry.source,EntrySource::Added(id) if id.get()==key))?;
    (!entry.facts.pickupable && entry.facts.bound && entry.facts.appearance_id == appearance)
        .then_some((pos, entry.facts))
}

// The shared claim proof authorizes a retained UID at the callback's exact
// selector. An object with a different action keeps that action's owner.
fn matches_legacy_chest_uid(
    map: &BundleMap,
    callback: &Callback,
    key: u64,
    appearance: u16,
) -> bool {
    map.facts.quest_callback_source_bound(key)
        && map.facts.action_at(key).is_none()
        && !map.facts.base_hidden(key)
        && world_callbacks::legacy_world_source_unique(callback).is_ok_and(|unique| {
            unique.is_some_and(|uid| {
                map.facts
                    .quest_callback_matches(key, Some(uid), None, Some(appearance))
            })
        })
}

pub(super) fn matches(map: &BundleMap, callback: &Callback, key: u64) -> bool {
    if callback.effects.loot_merchant
        || oteryn_gameplay_items::event_planting::world_change_item_source(key).is_some()
    {
        return shared_added_source(map, callback, key).is_some();
    }
    let Some((pos, ordinal)) = crate::map::view::placement_of(key) else {
        return false;
    };
    // Crystal actions_trap.lua bounds the encounter and its cleanup to this clearing.
    if callback.effects.place_item == Some(12187)
        && (ordinal != 0
            || pos.floor != -7
            || !(32651..=32668).contains(&pos.x)
            || !(32178..=32190).contains(&pos.y))
    {
        return false;
    }
    // Definition selectors are admitted through live Ground handles.
    if callback.selector.target_definition.is_some() {
        return false;
    }
    // These source-bound World ingresses retain no startup UID/AID in B3.
    // An appearance elsewhere in the stack cannot impersonate their source.
    let source_ordinal = match callback.key.as_str() {
        "oteryn:quest-callback/grimvale-full-moon-entry"
        | "oteryn:quest-callback/mages-tower-entry" => Some(1),
        "oteryn:quest-callback/mages-tower-exit" => Some(2),
        _ => None,
    };
    if source_ordinal.is_some_and(|source| source != ordinal) {
        return false;
    }
    if let Some(binding) = &callback.selector.native {
        // Eroth's authored route shares the donor crystal's retained AID with
        // the legacy route. Require the exact source action and native placement.
        let eroth_crystal = callback.key == EROTH_AUTHORED_MIRROR
            && callback.selector.action.is_some_and(|action| {
                map.facts
                    .quest_callback_matches(key, None, Some(action), Some(binding.appearance))
            })
            && !map.facts.house_tile(pos)
            && !map.facts.base_hidden(key);
        let matched = tile(binding.position) == pos
            && ordinal == binding.ordinal
            && map
                .overlay
                .base()
                .tile(pos.x, pos.y, pos.floor)
                .and_then(|base| {
                    base.ids()
                        .iter()
                        .zip(base.depths())
                        .filter(|(_, depth)| **depth == 0)
                        .nth(usize::from(ordinal))
                        .map(|(id, _)| *id)
                })
                .is_some_and(|id| {
                    eroth_crystal
                        || map
                            .facts
                            .quest_callback_native_object(key, id, binding.appearance)
                        || matches_legacy_chest_uid(map, callback, key, binding.appearance)
                });
        // Counterpart adapters retain a separately authoritative legacy route.
        // Native-only selectors must never become an unconstrained fallback.
        if matched
            || (callback.selector.position.is_none()
                && callback.selector.unique.is_none()
                && callback.selector.action.is_none())
        {
            return matched;
        }
    }
    if callback.selector.native.is_some()
        && callback
            .item_on_target
            .as_ref()
            .is_some_and(|item| item.selected_instance)
    {
        return false;
    }
    if oteryn_gameplay_quests::quest::shared_switches::native::bindings(&callback.key)
        .iter()
        .any(|binding| {
            tile(binding.position) == pos
                && ordinal == binding.ordinal
                && map
                    .overlay
                    .base()
                    .tile(pos.x, pos.y, pos.floor)
                    .and_then(|base| {
                        base.ids()
                            .iter()
                            .zip(base.depths())
                            .filter(|(_, depth)| **depth == 0)
                            .nth(usize::from(ordinal))
                            .map(|(id, _)| *id)
                    })
                    .is_some_and(|id| {
                        map.facts
                            .quest_callback_native_object(key, id, binding.appearance)
                    })
        })
    {
        return true;
    }
    if let Some(binding) =
        oteryn_gameplay_quests::quest::carried_step::native_binding(&callback.key)
        && tile(binding.position) == pos
        && ordinal == binding.ordinal
        && map.facts.quest_callback_native_teleport(
            key,
            binding.appearance,
            tile(binding.raw_destination),
        )
    {
        return true;
    }
    let selector = &callback.selector;
    // A native-only selector has no legacy alternative. Empty legacy terms
    // must not turn an exact source mismatch into a wildcard over every tile.
    if selector.position.is_none()
        && selector.unique.is_none()
        && selector.action.is_none()
        && selector.appearance.is_none()
    {
        return false;
    }
    // Crystal action_elevator.lua:18–52 switches between 2772 (travel) and
    // 2773 (reset) on the same placement. Bind the bare source lever once
    // and select the callback from its live appearance after each use.
    if callback
        .key
        .strip_prefix("oteryn:quest-callback/authored-new-frontier/")
        .is_some_and(|key| {
            matches!(
                key,
                "mine-lift"
                    | "mine-lift-reset"
                    | "mine-return"
                    | "mine-return-reset"
                    | "route-to-zao"
                    | "route-to-zao-reset"
                    | "route-to-farmine"
                    | "route-to-farmine-reset"
            )
        })
    {
        return selector.position
            == Some(Position {
                x: pos.x,
                y: pos.y,
                floor: pos.floor,
            })
            && selector.ordinal == Some(u16::from(ordinal))
            && map
                .overlay
                .base()
                .tile(pos.x, pos.y, pos.floor)
                .and_then(|base| {
                    base.ids()
                        .iter()
                        .zip(base.depths())
                        .filter(|(_, depth)| **depth == 0)
                        .nth(usize::from(ordinal))
                        .map(|(id, _)| *id)
                })
                .is_some_and(|id| {
                    [2772, 2773].into_iter().any(|appearance| {
                        map.facts.quest_callback_native_object(key, id, appearance)
                    }) && map
                        .facts
                        .base_entry(pos, ordinal, id)
                        .is_some_and(|entry| selector.appearance == Some(entry.appearance_id))
                });
    }
    if selector.unique.is_none()
        && selector.action.is_none()
        && map.facts.quest_callback_source_bound(key)
    {
        // Only the exact immutable World/legacy claim pair can share this UID.
        // Position and current appearance are still checked below.
        let Ok(Some(unique)) = world_callbacks::legacy_world_source_unique(callback) else {
            return false;
        };
        if !map
            .facts
            .quest_callback_matches(key, Some(unique), None, selector.appearance)
        {
            return false;
        }
    }
    if selector.ordinal.is_some()
        && selector.unique.is_none()
        && selector.action.is_none()
        && let Some(appearance) = selector.appearance
        && !map
            .overlay
            .base()
            .tile(pos.x, pos.y, pos.floor)
            .and_then(|base| {
                base.ids()
                    .iter()
                    .zip(base.depths())
                    .filter(|(_, depth)| **depth == 0)
                    .nth(usize::from(ordinal))
                    .map(|(id, _)| *id)
            })
            .is_some_and(|id| map.facts.quest_callback_native_object(key, id, appearance))
    {
        // An authored ordinal alias describes a bare native object. Retained
        // UID/AID bindings keep their existing source owner, including chests.
        return false;
    }
    // A position-registered USE acts on the resolved object the player clicked;
    // its native placement may be above ground. STEP still selects the floor.
    selector.matches_position(Position {
        x: pos.x,
        y: pos.y,
        floor: pos.floor,
    }) && selector
        .ordinal
        .is_none_or(|expected| expected == u16::from(ordinal))
        && (selector.position.is_none()
            || selector.unique.is_some()
            || selector.action.is_some()
            || selector.appearance.is_some()
            || callback.trigger == Trigger::Use
            || ordinal == 0)
        && (map
            .facts
            .quest_callback_matches(key, selector.unique, selector.action, None)
            || (selector.action.is_some()
                && selector.appearance.is_some_and(|appearance| {
                    matches_legacy_chest_uid(map, callback, key, appearance)
                })))
        && (map
            .facts
            .quest_callback_matches(key, None, None, selector.appearance)
            || ((selector.position.is_some()
                || callback.effects.place_item.is_some()
                || callback.key == oteryn_gameplay_quests::quest::rotten_blood::SACRIFICE
                || oteryn_gameplay_quests::quest::rotten_blood::flames::by_key(&callback.key)
                    .is_some())
                && selector.appearance.is_some_and(|appearance| {
                    // The Crystal plate callback is registered by item appearance. Plain
                    // placements are omitted from the sparse attribute index. Resolve
                    // their palette through the selected base stack, never a client item id.
                    map.overlay
                        .base()
                        .tile(pos.x, pos.y, pos.floor)
                        .and_then(|tile| {
                            tile.ids()
                                .iter()
                                .zip(tile.depths())
                                .filter(|(_, depth)| **depth == 0)
                                .nth(usize::from(ordinal))
                        })
                        .is_some_and(|(id, _)| {
                            map.facts.quest_callback_base_appearance(pos, ordinal, *id)
                                == Some(appearance)
                                || map
                                    .facts
                                    .base_entry(pos, ordinal, *id)
                                    .is_some_and(|facts| {
                                        facts.appearance_id == appearance
                                            || (callback.reversible
                                                && callback.lever.is_some_and(|pair| {
                                                    pair.contains(&appearance)
                                                        && pair.contains(&facts.appearance_id)
                                                }))
                                    })
                        })
                })))
}

pub(super) fn step_command(
    actor: ExactActorRef,
    session: GameSessionId,
    revision: u64,
    epoch: u64,
) -> Option<CommandRef> {
    use sha2::{Digest, Sha256};
    let digest = Sha256::new()
        .chain_update(b"oteryn:quest-world-step:v1")
        .chain_update(actor.placement_identity())
        .chain_update(epoch.to_be_bytes())
        .chain_update(revision.to_be_bytes())
        .finalize();
    let mut id = [0; 8];
    id.copy_from_slice(&digest[..8]);
    // The full actor, movement revision and epoch also enter the durable binding:
    // a compact-id collision refuses, rather than replaying another step's reward.
    CommandId::new(u64::from_be_bytes(id).max(1))
        .ok()
        .map(|id| CommandRef::new(session, id))
}

impl PendingCallbacks {
    pub(super) fn forget_ground_use(&self, session: GameSessionId) {
        if let Ok(mut uses) = self.ground_uses.lock() {
            uses.remove(&session);
        }
    }

    pub(super) fn finish_cancelled_ground_uses(&self, runtime: &mut ChannelRuntimeV1) {
        if let Ok(mut pending) = self.ground_use_cleanup.lock() {
            for turn in pending.drain(..) {
                turn.release(runtime);
            }
        }
    }
}

/// Once a callback starts a bounded commit whose result drives publication,
/// keep its reserved world state alive until that result is resolved.
async fn finish_quest_use(
    deadline: tokio::time::Instant,
    publication: &std::sync::atomic::AtomicBool,
    operation: impl Future<Output = UseDisposition>,
) -> UseDisposition {
    let mut operation = Box::pin(operation);
    match tokio::time::timeout_at(deadline, operation.as_mut()).await {
        Ok(result) => result,
        Err(_) if publication.load(std::sync::atomic::Ordering::Relaxed) => operation.await,
        Err(_) => UseDisposition::Rejected,
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Resolve a closed transport's reserved USE before the session stream can reopen.
    /// Historical receipt recovery does not grant player control or install a projection.
    pub(super) async fn reconcile_quest_callback_receipts(&self, session: GameSessionId) -> bool {
        if !self.reconcile_scene_receipts_for_session(session).await
            || !self.reconcile_ground_use_receipt(session).await
            || !self.reconcile_soul_war_reward_receipts(session).await
        {
            return false;
        }
        let pending = match self.states.quest_callbacks.records.lock() {
            Ok(records) => records
                .get(&session)
                .into_iter()
                .filter(|p| matches!(p.cause, QuestCause::Use(_)) && !p.ingress_settled)
                .cloned()
                .collect::<Vec<_>>(),
            Err(_) => return false,
        };
        if pending.is_empty() {
            return true;
        }
        if pending.iter().any(|pending| {
            callbacks().iter().any(|callback| {
                callback.key == pending.callback
                    && (callback.requires_live_premium()
                        || callback
                            .item_reward()
                            .as_ref()
                            .is_some_and(|reward| reward.premium))
            })
        }) {
            // Settle delivery before channel locks. A historical receipt still
            // replays when the current account gate is closed.
            let _ = self.session_premium_gate(session).await;
        }
        let Some(permit) = self.spell_lane_permit().await else {
            return false;
        };
        for pending in pending {
            let QuestCause::Use(command) = pending.cause else {
                return false;
            };
            let (Some(context), Some(catalogue)) = (pending.context, self.quest_catalogue.as_ref())
            else {
                return false;
            };
            // Replay the exact durable binding, including a bound carried tool.
            let context = match pending.selected_tool {
                Some((source, intent)) => match context.with_selected_tool(source, intent) {
                    Some(context) => context,
                    None => return false,
                },
                None => context.into(),
            }
            .with_paid_arrivals(pending.paid_arrivals);
            let Ok(Some(fence)) = self.current_quest_fence(pending.session).await else {
                return false;
            };
            let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
            let native = self.active_generation.and_then(|g| g.native_gameplay());
            let weights = native.map(|n| {
                crate::gameplay_transport::item_ref_admission::NativeItemWeights(n)
                    .with_wheel(self.premium_coordinator)
            });
            let admission = self
                .callback_content()
                .iter()
                .find(|c| c.key == pending.callback)
                .filter(|c| c.item_reward().is_some())
                .and_then(|_| native.zip(weights.as_ref()))
                .map(|(native, weights)| {
                    crate::durability::reward_claim_mint::QuestRewardAdmission {
                        actor_conditions: None,
                        carried_use: None,
                        creature_use: None,
                        peer_use: None,

                        native,
                        weights,
                        fee_payment: None,
                    }
                });
            let flame = pending.flame.clone();
            let outcome = if let Some(input) = pending.flame {
                slot.commit_rotten_flame_use(
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    input,
                    std::sync::Arc::clone(catalogue),
                    &permit,
                )
                .await
            } else {
                slot.commit_world_callback_with_items_selected(
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    QuestTransitionRequest {
                        transition_key: pending.callback,
                        cause: pending.cause,
                    },
                    std::sync::Arc::clone(catalogue),
                    &permit,
                    context,
                    self.active_generation
                        .and_then(|generation| generation.native_gameplay()),
                    admission,
                    &self.premium_coordinator,
                )
                .await
            };
            let disposition = match outcome {

                Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => UseDisposition::Committed,
                Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(_)) => return false,
                Ok(QuestTransitionOutcome::Refused(oteryn_gameplay_quests::quest::QuestRefusal::StageMismatch))
                    if flame.as_ref().and_then(|input|input.no_change()).is_some()=>UseDisposition::Committed,
                Ok(_) | Err(_) => UseDisposition::Rejected,
            };
            if self
                .states
                .command_ingress
                .complete_quest_callback(command, disposition)
                .is_none()
            {
                return false;
            }
            let Ok(mut records) = self.states.quest_callbacks.records.lock() else {
                return false;
            };
            if disposition == UseDisposition::Committed {
                if let Some(record) = records.get_mut(&pending.session) {
                    record.ingress_settled = true;
                }
            } else {
                records.remove(&pending.session);
            }
        }
        true
    }

    #[cfg(test)]
    pub(super) fn forget_coffin_cohort_for_test(&self) -> Result<(), &'static str> {
        self.states.quest_callbacks.personal.clear_for_test()
    }

    #[cfg(test)]
    pub(super) fn forget_coffin_projection_for_test(
        &self,
        session: GameSessionId,
    ) -> Result<(), &'static str> {
        let mut pending = self
            .states
            .quest_callbacks
            .records
            .lock()
            .map_err(|_| "pending")?;
        pending
            .remove(&session)
            .ok_or("no interrupted projection")?;
        Ok(())
    }

    pub(super) fn has_pending_quest_callback(&self, session: GameSessionId) -> bool {
        if self.states.grizzly_boss.has_pending(session)
            || self
                .states
                .quest_callbacks
                .scene_item_receipts
                .has_session(session)
            || self
                .states
                .quest_callbacks
                .ground_uses
                .lock()
                .map_or(true, |uses| uses.contains_key(&session))
        {
            return true;
        }
        self.states
            .quest_callbacks
            .soul_rewards
            .lock()
            .map_or(true, |p| p.contains_key(&session))
            || self
                .states
                .quest_callbacks
                .records
                .lock()
                .map_or(true, |p| {
                    p.contains_key(&session)
                        || p.values()
                            .any(|pending| pending.participants.contains(&session))
                })
    }

    /// A debit receipt belongs to the original World occurrence. Keep the
    /// actor and its reservation available until that occurrence is projected.
    pub(super) fn has_pending_paid_quest_callback(&self, session: GameSessionId) -> bool {
        self.states
            .quest_callbacks
            .records
            .lock()
            .map_or(true, |records| {
                records.values().any(|pending| {
                    (pending.session == session || pending.participants.contains(&session))
                        && self
                            .callback_content()
                            .iter()
                            .any(|callback| callback.key == pending.callback && may_debit(callback))
                })
            })
    }

    pub(super) async fn settle_paid_quest_callbacks(&self, session: GameSessionId) -> bool {
        if self.has_pending_paid_quest_callback(session) {
            Box::pin(self.reconcile_quest_callbacks(session)).await
                && !self.has_pending_paid_quest_callback(session)
        } else {
            true
        }
    }

    /// Called with the common node input gate quiesced. Enumerate the
    /// initiators directly so an untracked actor cannot strand a party payment.
    pub(super) async fn drain_paid_quest_callbacks(&self) -> usize {
        let sessions = match self.states.quest_callbacks.records.lock() {
            Ok(records) => records
                .values()
                .filter_map(|pending| {
                    self.callback_content()
                        .iter()
                        .any(|callback| callback.key == pending.callback && may_debit(callback))
                        .then_some(pending.session)
                })
                .collect::<Vec<_>>(),
            Err(_) => return 1,
        };
        let mut failed = 0;
        for session in sessions {
            if !self.settle_paid_quest_callbacks(session).await {
                failed += 1;
            }
        }
        failed
    }

    fn paid_callback_can_start(
        &self,
        session: GameSessionId,
        initiator: GameSessionId,
        source: impl FnOnce(&Pending) -> bool,
    ) -> bool {
        let callbacks = &self.states.quest_callbacks;
        !callbacks.closing(session)
            || callbacks
                .records
                .lock()
                .is_ok_and(|records| records.get(&initiator).is_some_and(source))
    }

    pub(super) fn callback_content(&self) -> &[Callback] {
        #[cfg(test)]
        if let Some(content) = self.states.quest_callbacks.fixture.get() {
            return content;
        }
        if let Some(callbacks) = self
            .quest_catalogue
            .as_deref()
            .and_then(|c| c.authored_world_callbacks())
        {
            return callbacks;
        }
        callbacks()
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "native source identity and current source position remain explicit"
    )]
    async fn observe_stateless_world_callback(
        &self,
        permit: &SpellLanePermit,
        fence: Option<CurrentCharacterGameplayFence>,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        revision: u64,
        expected: Option<MovementPositionSnapshot>,
    ) -> Result<Option<ObservedWorldCallback>, UseDisposition> {
        if callback
            .world_requirements
            .iter()
            .any(|r| r.ground_stack.is_some())
        {
            let index = {
                let runtime = self.runtime.lock().await;
                super::item_ref_admission::pinned_index(&runtime, self.active_generation)
            };
            if super::loop_move::loop_ground(self, map.overlay.base(), index)
                .await
                .is_none()
            {
                return Err(UseDisposition::Rejected);
            }
        }
        if !stateless_observation_required(callback) {
            return Ok(None);
        }
        let Some(fence) = fence else {
            return Err(UseDisposition::Rejected);
        };
        let (snapshot, context) = {
            let runtime = self.runtime.lock().await;
            let snapshot = runtime
                .read_actor_position(actor)
                .map_err(|_| UseDisposition::Rejected)?;
            if runtime.owner_fence().is_err()
                || expected.is_some_and(|expected| expected != snapshot)
                || map.facts.object_revision(key) != revision
                || !matches(map, callback, key)
                || !runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return Err(UseDisposition::StaleState);
            }
            let context = QuestWorldContext {
                party: Vec::new(),
                map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                content_revision: format!(
                    "sha256:{}",
                    hex(&runtime.content_pin().server_artifact_digest())
                ),
                placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                reset_epoch: self.reset_epoch,
                actor_placement: actor.placement_identity(),
                movement_revision: snapshot.revision(),
            };
            (snapshot, context)
        };
        let backend_callback = native(snapshot.position()).map_or_else(
            || callback.key.clone(),
            |at| {
                oteryn_gameplay_quests::quest::hive_access::observation_key(
                    &callback.key,
                    Position {
                        x: at.x,
                        y: at.y,
                        floor: at.floor,
                    },
                )
            },
        );
        // No runtime or spell-state guard is retained over this independent current query.
        // Only the derived observation key bypasses stage/scent for egress. The actual
        // callback key, selector, movement snapshot and source revision stay unchanged.
        if !matches!(
            self.root
                .observe_world_callback_gate(
                    self.character,
                    self.holder,
                    permit,
                    fence,
                    context.clone(),
                    backend_callback,
                )
                .await,
            Ok(true)
        ) {
            return Err(UseDisposition::Rejected);
        }
        Ok(Some(ObservedWorldCallback {
            actor,
            session,
            expected: snapshot,
            key,
            revision,
            callback: callback.key.clone(),
            context,
        }))
    }

    /// Before further player input, reconcile an interrupted commit with its original cause.
    /// The Channel's existing input gate closes a connection if a result is still unknown.
    pub(super) async fn reconcile_quest_callbacks(&self, session: GameSessionId) -> bool {
        if !self.reconcile_grizzly_boss(session).await {
            return false;
        }
        let session = match self.states.quest_callbacks.records.lock() {
            Ok(records) => records
                .iter()
                .find_map(|(owner, pending)| {
                    pending.participants.contains(&session).then_some(*owner)
                })
                .unwrap_or(session),
            Err(_) => return false,
        };
        if !self.reconcile_quest_callback_receipts(session).await {
            return false;
        }
        let pending = match self.states.quest_callbacks.records.lock() {
            Ok(p) => p.get(&session).cloned().into_iter().collect::<Vec<_>>(),
            Err(_) => return false,
        };
        if pending.is_empty() {
            return true;
        }
        let Some(permit) = self.spell_lane_permit().await else {
            return false;
        };
        for pending in pending {
            let Some(map) = self.bundle_map() else {
                return false;
            };
            let Some(callback) = self
                .callback_content()
                .iter()
                .find(|c| c.key == pending.callback)
            else {
                return false;
            };
            let Ok(Some(fence)) = self.current_quest_fence(pending.session).await else {
                return false;
            };
            let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
            let observation = self
                .observe_stateless_world_callback(
                    &permit,
                    Some(fence),
                    pending.actor,
                    pending.session,
                    map,
                    callback,
                    pending.key,
                    pending.revision,
                    Some(pending.expected),
                )
                .await;
            if stateless_observation_required(callback) {
                // A timed-out read has no unknown personal value/receipt to reconcile.
                // Its now-known result follows the normal native return-on-failure path.
                let Ok(mut records) = self.states.quest_callbacks.records.lock() else {
                    return false;
                };
                records.remove(&pending.session);
            }
            let optional_pvp = self
                .special_movement_policy(&permit, pending.actor, pending.session)
                .await;
            let mut guards = Some(self.lock_callback_guards().await);
            let Some((runtime, _)) = guards.as_mut() else {
                return false;
            };
            if runtime.read_actor_position(pending.actor) != Ok(pending.expected) {
                return false;
            }
            let result = match observation {
                Ok(observation) => {
                    Box::pin(self.execute_quest_callback(
                        &permit,
                        &mut guards,
                        pending.actor,
                        pending.session,
                        map,
                        callback,
                        pending.key,
                        pending.revision,
                        pending.cause,
                        Some(fence),
                        Some(&mut slot),
                        observation.as_ref(),
                        optional_pvp,
                        None,
                        pending.selected_tool.as_ref(),
                        None,
                    ))
                    .await
                }
                Err(disposition) => disposition,
            };
            let Some((runtime, states)) = guards.as_mut() else {
                return false;
            };
            if result != UseDisposition::Committed {
                if self.has_pending_quest_callback(pending.session) {
                    return false;
                }
                if let Some(origin) = pending.origin {
                    self.refuse_quest_step(
                        optional_pvp,
                        runtime,
                        states,
                        pending.actor,
                        pending.session,
                        map,
                        callback,
                        pending.expected,
                        origin,
                    );
                }
            }
        }
        true
    }

    /// Both bare source USE and a carried tool on a source use the existing
    /// callback receipt owner, after the wire handle owner resolved the real ID.
    pub(super) async fn use_shared_callback_handle(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        source: Option<super::item_view::ItemKey>,
        target: Option<super::item_view::ItemKey>,
    ) -> Option<UseOutcome> {
        use super::item_view::ItemKey;
        use crate::map::view::MapFacts;
        let key = match (source, target) {
            (_, Some(ItemKey::MapAdded(key))) => key,
            (Some(ItemKey::MapAdded(key)), None) => key,
            _ => return None,
        };
        let map = self.bundle_map()?;
        let (pos, _) = map.facts.movement_item(key)?;
        let callback = self
            .callback_content()
            .iter()
            .find(|callback| shared_added_source(map, callback, key).is_some())?;
        if callback.item_on_target.is_some() != target.is_some() {
            return Some(UseOutcome::rejected());
        }
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let _ = pos;
        self.use_quest_callback(
            actor,
            command,
            map,
            &WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            },
        )
        .await
    }

    async fn lock_callback_guards(&self) -> CallbackGuards<'_> {
        super::lock_runtime_and_spell_states(&self.runtime, &self.states.spell_states).await
    }

    pub(super) async fn use_quest_callback(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        self.use_quest_callback_with_tool(actor, command, map, target, None)
            .await
    }

    pub(super) async fn use_quest_callback_with_tool(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        target: &WorldObjectTarget,
        selected_tool: Option<([u8; 16], Vec<u8>)>,
    ) -> Option<UseOutcome> {
        if let Some(outcome) = self.use_rafzane_elevator(actor, command, map, target).await {
            return Some(outcome);
        }
        if let Some(outcome) = self.use_soul_war_reward(actor, command, map, target).await {
            return Some(outcome);
        }
        let source = super::world_map::MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: self.active_generation?.identity().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let added_key = target
            .placement
            .get(32..40)
            .and_then(|bytes| <[u8; 8]>::try_from(bytes).ok())
            .map(u64::from_be_bytes)
            .filter(|key| {
                oteryn_gameplay_items::event_planting::world_change_item_source(*key).is_some()
                    || captured_merchant(map, *key).is_some()
            });
        let resolved = if let Some(key) = added_key {
            use crate::map::view::MapFacts;
            if target.placement.len() != 40
                || target.placement[..32] != map.overlay.base().digest()
                || target.expected_revision != map.facts.object_revision(key)
            {
                return Some(UseOutcome::rejected());
            }
            let (pos, _) = map.facts.movement_item(key)?;
            super::world_map::MapObjectTarget {
                pos,
                ordinal: 0,
                placement_key: key,
                canonical: None::<()>,
            }
        } else {
            super::world_map::resolve_map_target(&source, target, |_| None::<()>).ok()?
        };
        if !self
            .reconcile_pending_scene_items(map, resolved.placement_key)
            .await
        {
            return Some(UseOutcome::rejected());
        }
        let callbacks = self.callback_content();
        let mut selected = callbacks
            .iter()
            .filter(|c| {
                c.actor_kind == world_callbacks::ActorKind::Player
                    && c.trigger == Trigger::Use
                    && !superseded_chest(c, callbacks)
                    && !oteryn_gameplay_quests::quest::ice_seal_paint::historical_only(&c.key)
                    && matches(map, c, resolved.placement_key)
                    && c.activation.as_ref().is_none_or(|activation| {
                        self.states
                            .quest_sessions
                            .lock()
                            .ok()
                            .and_then(|sessions| {
                                sessions
                                    .get(&command.game_session_id)
                                    .and_then(|s| s.copy.as_ref())
                                    .and_then(|copy| {
                                        copy.read_tracks(self.quest_catalogue.as_deref()?)?
                                            .get(&activation.track)
                                            .copied()
                                    })
                            })
                            .is_some_and(|value| {
                                (activation.minimum..=activation.maximum).contains(&value)
                            })
                    })
                    && conditional_cells::matches(map, c)
            })
            .collect::<Vec<_>>();
        if let [left, right] = selected.as_slice()
            && world_callbacks::phase::hoard_reentry_pair(left, right)
        {
            // A bare hoard USE only resumes a paid entry. The selected tool route
            // can charge only while the fenced state still awaits payment.
            selected.retain(|callback| callback.is_read_only() == selected_tool.is_none());
        }
        let source_trial = selected
            .iter()
            .any(|c| c.effects.target_transform.is_some());
        self.select_source_phase(&mut selected, actor, &command)
            .await;
        if source_trial && selected.is_empty() {
            return Some(UseOutcome::rejected());
        }
        if selected.len() > 1
            && selected.iter().all(|callback| {
                callback
                    .item_on_target
                    .as_ref()
                    .is_some_and(|item| item.selected_instance)
            })
        {
            let (Some(fence), Some(catalogue)) =
                (command.item_fence, self.quest_catalogue.as_deref())
            else {
                return Some(UseOutcome::rejected());
            };
            let Ok(snapshot) = self
                .root
                .read_character_quest_state(self.character, fence.character_id)
                .await
            else {
                return Some(UseOutcome::rejected());
            };
            selected.retain(|callback| {
                callback.storage.iter().all(|state| {
                    snapshot
                        .tracks()
                        .get(&state.track)
                        .copied()
                        .or_else(|| catalogue.track(&state.track).map(|track| track.initial))
                        .is_some_and(|value| (state.minimum..=state.maximum).contains(&value))
                })
            });
        }
        let mut selection_permit = None;
        let mut selection_deadline = None;
        if let [first, second] = selected.as_slice()
            && let Some((ordinary, weakened)) =
                if first.vampire_coffin_fallback() == Some(second.key.as_str()) {
                    Some((*second, *first))
                } else if second.vampire_coffin_fallback() == Some(first.key.as_str()) {
                    Some((*first, *second))
                } else {
                    None
                }
        {
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
            let selection = tokio::time::timeout_at(deadline, async {
                let permit = self.spell_lane_permit().await?;
                let (index, generation, content, placement) = {
                    let runtime = self.runtime.lock().await;
                    permit.check_scope(runtime.binding().runtime_scope()).ok()?;
                    (
                        super::item_ref_admission::pinned_index(&runtime, self.active_generation)?,
                        runtime.binding().scope_generation().get(),
                        format!("sha256:{}", hex(&runtime.content_pin().server_artifact_digest())),
                        runtime.content_pin().frame_binding_digest(),
                    )
                };
                let pending = self.root.read_pending_vampire_coffin(
                    self.character, command.item_fence?.character_id,
                ).await.ok()?;
                if let Some(kind) = pending {
                    return Some((permit, match kind {
                        VampireCoffinKind::Ordinary => ordinary,
                        VampireCoffinKind::Weakened => weakened,
                    }));
                }
                let ground = super::loop_move::loop_ground(self, map.overlay.base(), Some(index)).await?;
                let has_garlic = weakened.offerings.iter().all(|input| {
                    ground.overlay().tile(tile(input.position)).is_some_and(|tile| {
                        tile.added().iter().any(|item| {
                            matches!(item.item(), crate::map::overlay::AddedItem::Ground(item)
                                if item.runtime_scope_ownership_generation == generation
                                    && item.ground.content_revision == content
                                    && item.ground.native_room_placement_context == placement
                                    && item.definition.production_key == input.item
                                    && item.quantity >= u32::from(input.count)
                                    && item.ground.corpse_ref.starts_with(
                                        crate::durability::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX))
                        })
                    })
                });
                // The durable participant rechecks the selected variant and its inputs.
                Some((permit, if has_garlic { weakened } else { ordinary }))
            }).await;
            let Ok(Some((permit, callback))) = selection else {
                return Some(UseOutcome::rejected());
            };
            selected = vec![callback];
            selection_permit = Some(permit);
            selection_deadline = Some(deadline);
        }
        if selected.len() > 1
            && selected.iter().all(|callback| {
                !callback.requires_live_premium()
                    && callback
                        .item_reward()
                        .as_ref()
                        .is_none_or(|reward| !reward.premium)
                    && callback.world_requirements.iter().any(|requirement| {
                        requirement.ground.is_some() || requirement.ground_stack.is_some()
                    })
            })
        {
            // Completed wire commands replay before this fresh route; pending
            // reconciliation uses its retained callback identity. Ground is only
            // selection advice here: both final participants still recheck it.
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
            let permit = tokio::time::timeout_at(deadline, async {
                let permit = self.spell_lane_permit().await?;
                if !self
                    .retain_ground_candidates(&permit, map, &mut selected)
                    .await
                    || selected.is_empty()
                {
                    return None;
                }
                Some(permit)
            })
            .await;
            let Ok(Some(permit)) = permit else {
                return Some(UseOutcome::rejected());
            };
            selection_permit = Some(permit);
            selection_deadline = Some(deadline);
        }
        let callback = if let [callback] = selected.as_slice() {
            self.serpentine_journey_opening(callback, map, &resolved, command.game_session_id)
                .await
                .unwrap_or(callback)
        } else {
            if selected.is_empty() {
                return None;
            }
            let Ok(sessions) = self.states.quest_sessions.lock() else {
                return Some(UseOutcome::rejected());
            };
            let Some(copy) = sessions
                .get(&command.game_session_id)
                .and_then(|state| state.copy.as_ref())
            else {
                return Some(UseOutcome::rejected());
            };
            let Some(tracks) = self
                .quest_catalogue
                .as_deref()
                .and_then(|catalogue| copy.read_tracks(catalogue))
            else {
                return Some(UseOutcome::rejected());
            };
            use oteryn_gameplay_quests::quest::koshei_amulet::{SUNDIAL, TOMB_ENTRY};
            // The two Koshei callbacks deliberately share the same sundial. A
            // ready statue puzzle opens the coffin first. Otherwise retain
            // the authored encounter eligibility and the puzzle's refusal.
            let koshei = match selected.as_slice() {
                [first, second] if first.key == SUNDIAL && second.key == TOMB_ENTRY => {
                    Some((*first, *second))
                }
                [first, second] if first.key == TOMB_ENTRY && second.key == SUNDIAL => {
                    Some((*second, *first))
                }
                _ => None,
            };
            let resolved = if let Some((sundial, entry)) = koshei {
                if world_requirements_match(map, sundial) {
                    Some(sundial)
                } else {
                    unique_eligible(std::iter::once(entry), &tracks).or(Some(sundial))
                }
            } else {
                unique_eligible(selected.iter().copied(), &tracks)
            };
            let Some(callback) = resolved else {
                return Some(UseOutcome::rejected());
            };
            callback
        };
        if callback.requires_live_premium()
            || callback
                .item_reward()
                .as_ref()
                .is_some_and(|reward| reward.premium)
        {
            // The synchronous live view used by MINT needs delivery settled;
            // await that read before taking the spell lane and Channel guards.
            // Do not refuse here: exact historical receipts remain replayable.
            let _ = self.session_premium_gate(command.game_session_id).await;
        }
        let deadline = selection_deadline
            .unwrap_or_else(|| tokio::time::Instant::now() + std::time::Duration::from_secs(10));
        let publication = std::sync::atomic::AtomicBool::new(false);
        let result = finish_quest_use(deadline, &publication, async {
            let permit = match selection_permit {
                Some(permit) => permit,
                None => match self.spell_lane_permit().await {
                    Some(permit) => permit,
                    None => return UseDisposition::Rejected,
                },
            };
            let fence = if character_fence_required(callback) {
                self.current_quest_fence(command.game_session_id).await.ok().flatten()
            } else { None };
            if fence.is_some_and(|f| command.item_fence != Some(f.item_fence())) {
                return UseDisposition::Rejected;
            }
            let mut slot = match fence {
                Some(f) => Some(self.revision_sequencer.acquire(f.character_id).await),
                None => None,
            };
            let observation=match self.observe_stateless_world_callback(
                &permit,fence,actor,command.game_session_id,map,callback,resolved.placement_key,target.expected_revision,None,
            ).await {Ok(observation)=>observation,Err(disposition)=>return disposition};
            let optional_pvp = self
                .special_movement_policy(&permit, actor, command.game_session_id)
                .await;
            let read_only_gate = if callback.is_read_only() {
                let (Some(fence), Some(catalogue)) = (fence, self.quest_catalogue.as_ref()) else {
                    return UseDisposition::Rejected;
                };
                let (expected, context) = {
                    let runtime = self.runtime.lock().await;
                    let Ok(expected) = runtime.read_actor_position(actor) else {
                        return UseDisposition::Rejected;
                    };
                    let Some(at) = native(expected.position()) else {
                        return UseDisposition::Rejected;
                    };
                    if !callback.reaches(Position { x: at.x, y: at.y, floor: at.floor }, Position { x: resolved.pos.x, y: resolved.pos.y, floor: resolved.pos.floor }) {
                        return UseDisposition::TooFar;
                    }
                    if !self.states.world_gate.lock()
                        .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
                    {
                        return UseDisposition::Exhausted;
                    }
                    let context = QuestWorldContext {
                        party: Vec::new(),
                        map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                        content_revision: format!("sha256:{}", hex(&runtime.content_pin().server_artifact_digest())),
                        placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                        reset_epoch: self.reset_epoch,
                        actor_placement: actor.placement_identity(),
                        movement_revision: expected.revision(),
                    };
                    (expected, context)
                };
                let allowed = self.root.check_character_world_callback(
                    self.character, self.holder, fence, &callback.key,
                    std::sync::Arc::clone(catalogue), &permit, context,
                ).await;
                if !matches!(allowed, Ok(true)) {
                    if let Some(message) = &callback.failure_message {
                        self.states.chat.push_quest_feedback(command.game_session_id, message);
                    }
                    return UseDisposition::Rejected;
                }
                Some(ReadOnlyGate {
                    actor, session: command.game_session_id, expected, callback: callback.key.clone(),
                })
            } else {
                None
            };
            let runtime = self.runtime.lock().await;
            let Ok(expected) = runtime.read_actor_position(actor) else {
                return UseDisposition::Rejected;
            };
            let Some(at) = native(expected.position()) else {
                return UseDisposition::Rejected;
            };
            if !callback.reaches(Position { x: at.x, y: at.y, floor: at.floor }, Position { x: resolved.pos.x, y: resolved.pos.y, floor: resolved.pos.floor }) {
                return UseDisposition::TooFar;
            }
            if changes_carried_items(callback)
                && !self.states.player_trades.physical_service(command.game_session_id)
            {
                return UseDisposition::Rejected;
            }
            if !callback.is_read_only() && !self
                .states
                .world_gate
                .lock()
                .is_ok_and(|mut g| if selected_tool.is_some() {
                    g.admit_use_with(actor, self.clock_origin.elapsed())
                } else {
                    g.admit(actor, self.clock_origin.elapsed())
                })
            {
                return UseDisposition::Exhausted;
            }
            let Ok(id) = CommandId::new(command.command_id) else {
                return UseDisposition::Rejected;
            };
            let mut directional;
            let callback = if oteryn_gameplay_quests::quest::hive_access::gate(&callback.key).is_some() {
                directional = (*callback).clone();
                let Some(destination)=oteryn_gameplay_quests::quest::hive_access::destination(&callback.key,
                    Position {x:at.x,y:at.y,floor:at.floor}) else { return UseDisposition::Rejected; };
                directional.teleport=Some(destination);
                &directional
            } else { callback };
            drop(runtime);
            let scene_items = match self.prepare_scene_items(
                &permit, actor, command, map, callback, resolved.placement_key, target.expected_revision, fence,
            ).await {
                Ok(items) => items,
                Err(disposition) => return disposition,
            };
            let mut guards = Some(self.lock_callback_guards().await);
            if scene_items.is_some() || VampireCoffinKind::for_callback(&callback.key).is_some() {
                publication.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            let mut scene_publication = None;
            let original_use = CommandRef::new(command.game_session_id, id);
            let result = Box::pin(self.execute_quest_callback(
                &permit,
                &mut guards,
                actor,
                command.game_session_id,
                map,
                callback,
                resolved.placement_key,
                target.expected_revision,
                QuestCause::Use(original_use),
                fence,
                slot.as_mut(),
                observation.as_ref(),
                optional_pvp,
                read_only_gate.as_ref(),
                selected_tool.as_ref(),
                scene_items.as_ref().map(|_| &mut scene_publication),
            ))
            .await;
            drop(guards);
            drop(slot);
            if let (Some(items), Some(scene)) = (scene_items, scene_publication) {
                return self.commit_scene_items(
                    permit, map, callback, resolved.placement_key, actor, original_use, optional_pvp, items, scene,
                ).await;
            }
            if matches!(result, UseDisposition::Committed | UseDisposition::Rejected)
                && let Some(fence) = fence
                && let Some(delivery) = self.deliver_monk_reward(actor, command, callback, fence).await
            {
                if delivery == UseDisposition::Committed { return delivery; }
                if result == UseDisposition::Committed {
                    self.states.chat.push_quest_feedback(command.game_session_id,
                        "Your shrine reward is earned. Make room and capacity in your backpack and use this shrine again to collect it.");
                    return result;
                }
                return delivery;
            }
            result
        })
        .await;
        Some(UseOutcome {
            disposition: result,
            ..UseOutcome::rejected()
        })
    }

    /// The ordinary step already holds the spell lane. Release its runtime guard before
    /// acquiring the quest revision slot, then compare the movement snapshot on re-entry.
    pub(super) async fn step_quest_callback(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        let entered = if origin.position() != entered.position() {
            self.step_quest_trigger(permit, actor, session, origin, entered, Trigger::StepOut)
                .await
        } else {
            entered
        };
        let entered = self
            .step_arena_portal(permit, actor, session, origin, entered)
            .await;
        let entered = self.step_arena_exit(permit, actor, session, entered).await;
        let entered = self
            .step_dynamic_quest_portal(permit, actor, session, entered)
            .await;
        self.step_quest_trigger(permit, actor, session, origin, entered, Trigger::StepIn)
            .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "binds the exact actor, session, movement snapshots and source trigger"
    )]
    async fn step_quest_trigger(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
        trigger: Trigger,
    ) -> MovementPositionSnapshot {
        let Some(map) = self.bundle_map() else {
            return entered;
        };
        let selected_position = if trigger == Trigger::StepOut {
            origin.position()
        } else {
            entered.position()
        };
        let Some(at) = native(selected_position) else {
            return entered;
        };
        let Some(base) = map.overlay.base().tile(at.x, at.y, at.floor) else {
            return entered;
        };
        let Some(command) = step_command(actor, session, entered.revision(), self.reset_epoch)
        else {
            return entered;
        };
        let cause = QuestCause::Step(command);
        let selected = (0..base.depths().iter().filter(|d| **d == 0).count())
            .filter_map(|o| {
                let key = placement_key(at, u8::try_from(o).ok()?)?;
                if map.facts.base_hidden(key) {
                    return None;
                }
                Some(
                    self.callback_content()
                        .iter()
                        .filter(move |c| {
                            c.actor_kind == world_callbacks::ActorKind::Player
                                && c.trigger == trigger
                                && matches(map, c, key)
                        })
                        .map(move |c| (c, key)),
                )
            })
            .flatten()
            .collect::<Vec<_>>();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut selected = selected;
        if selected.len() > 1
            && selected.iter().enumerate().all(|(i, (left, _))| {
                selected[..i]
                    .iter()
                    .all(|(right, _)| world_callbacks::phase::exclusive(left, right))
            })
        {
            // The gated action rechecks the selected phase under the Character fence.
            let preview = async {
                let key = selected[0].1;
                if selected.iter().any(|(_, candidate)| *candidate != key) {
                    return None;
                }
                let catalogue = self.quest_catalogue.as_deref()?;
                let fence = self.current_quest_fence(session).await.ok()??;
                #[cfg(test)]
                if self
                    .states
                    .quest_callbacks
                    .phase_read_unavailable
                    .swap(false, std::sync::atomic::Ordering::AcqRel)
                {
                    return None;
                }
                let state = self
                    .root
                    .read_character_quest_state(self.character, fence.character_id)
                    .await
                    .ok()?;
                let candidates = selected
                    .iter()
                    .map(|(callback, _)| *callback)
                    .collect::<Vec<_>>();
                let mut tracks = state.read_tracks(catalogue)?;
                if let Some(pin) = state.states().get(&candidates[0].quest)
                    && let Some(defaults) =
                        oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(
                            catalogue,
                            &candidates[0].quest,
                            &pin.definition_hash,
                        )
                {
                    for (track, value) in defaults {
                        tracks.entry(track.clone()).or_insert(*value);
                    }
                }
                use world_callbacks::phase;
                if candidates.iter().any(|candidate| {
                    phase::sea_ear_route(candidate).is_some()
                        && candidate
                            .storage
                            .iter()
                            .filter(|gate| gate.track != phase::SEA_EAR_TRACK)
                            .all(|gate| {
                                tracks.get(&gate.track).is_some_and(|value| {
                                    (gate.minimum..=gate.maximum).contains(value)
                                })
                            })
                }) {
                    let inventory = self
                        .root
                        .read_item_use_inventory(self.character, self.holder, fence.item_fence())
                        .await
                        .ok()?;
                    let carried = inventory
                        .equipment
                        .iter()
                        .chain(
                            inventory
                                .backpack
                                .iter()
                                .flat_map(|bag| bag.entries.iter())
                                .map(|entry| &entry.item),
                        )
                        .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
                        .any(|item| item.definition.production_key == phase::SEA_EAR_ITEM);
                    tracks.insert(phase::SEA_EAR_TRACK.into(), if carried { 1 } else { -1 });
                }
                let chosen = world_callbacks::select_storage_phase(&candidates, &tracks, catalogue)
                    .ok()??;
                Some((chosen, key))
            };
            match tokio::time::timeout_at(deadline, preview).await {
                Ok(Some(pair)) => {
                    selected.clear();
                    selected.push(pair);
                }
                _ => {
                    if selected
                        .iter()
                        .all(|(callback, _)| callback.return_on_failure)
                    {
                        return self
                            .rollback_quest_step(
                                actor,
                                session,
                                map,
                                selected[0].0,
                                entered,
                                origin,
                            )
                            .await;
                    }
                    return entered;
                }
            }
        }
        let had_callbacks = !selected.is_empty();
        self.select_step_phase(&mut selected, actor, session, entered, &cause);
        // Ambiguous content never chooses one action by incidental registration order.
        if selected.is_empty() {
            if !had_callbacks && trigger == Trigger::StepIn {
                return self
                    .step_daily_world_gate(permit, actor, session, origin, entered)
                    .await;
            }
            return entered;
        }
        let selected_pair = if let [pair] = selected.as_slice() {
            *pair
        } else {
            let Ok(sessions) = self.states.quest_sessions.lock() else {
                return entered;
            };
            let Some(copy) = sessions.get(&session).and_then(|state| state.copy.as_ref()) else {
                return entered;
            };
            let Some(tracks) = self
                .quest_catalogue
                .as_deref()
                .and_then(|catalogue| copy.read_tracks(catalogue))
            else {
                return entered;
            };
            let Some(callback) =
                unique_eligible(selected.iter().map(|(callback, _)| *callback), &tracks)
            else {
                return entered;
            };
            let Some(pair) = selected
                .iter()
                .find(|(candidate, _)| std::ptr::eq(*candidate, callback))
            else {
                return entered;
            };
            *pair
        };
        let (callback, key) = &selected_pair;
        if callback.requires_live_premium() {
            let _ = self.session_premium_gate(session).await;
        }
        let operation = async {
            let fence = if character_fence_required(callback) {
                self.current_quest_fence(session).await.ok().flatten()
            } else {
                None
            };
            let mut slot = match fence {
                Some(f) => Some(self.revision_sequencer.acquire(f.character_id).await),
                None => None,
            };
            let read_only_gate = if callback.is_read_only() {
                if let (Some(fence), Some(catalogue)) = (fence, self.quest_catalogue.as_ref()) {
                    let context = {
                        let runtime = self.runtime.lock().await;
                        (runtime.read_actor_position(actor) == Ok(entered)).then(|| {
                            QuestWorldContext {
                                map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                                content_revision: format!(
                                    "sha256:{}",
                                    hex(&runtime.content_pin().server_artifact_digest())
                                ),
                                placement_context: runtime
                                    .content_pin()
                                    .frame_binding_digest()
                                    .to_vec(),
                                reset_epoch: self.reset_epoch,
                                actor_placement: actor.placement_identity(),
                                movement_revision: entered.revision(),
                                party: Vec::new(),
                            }
                        })
                    };
                    if let Some(context) = context {
                        matches!(
                            self.root
                                .check_character_world_callback(
                                    self.character,
                                    self.holder,
                                    fence,
                                    &callback.key,
                                    std::sync::Arc::clone(catalogue),
                                    permit,
                                    context,
                                )
                                .await,
                            Ok(true)
                        )
                        .then(|| ReadOnlyGate {
                            actor,
                            session,
                            expected: entered,
                            callback: callback.key.clone(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };
            let revision = map.facts.object_revision(*key);
            let observation = self
                .observe_stateless_world_callback(
                    permit,
                    fence,
                    actor,
                    session,
                    map,
                    callback,
                    *key,
                    revision,
                    Some(entered),
                )
                .await;
            let optional_pvp = self.special_movement_policy(permit, actor, session).await;
            let mut guards = Some(self.lock_callback_guards().await);
            let Some((runtime, _)) = guards.as_mut() else {
                return entered;
            };
            if runtime.read_actor_position(actor) != Ok(entered) {
                return entered;
            }
            let result = match observation {
                Ok(observation) => {
                    Box::pin(self.execute_quest_callback(
                        permit,
                        &mut guards,
                        actor,
                        session,
                        map,
                        callback,
                        *key,
                        revision,
                        cause,
                        fence,
                        slot.as_mut(),
                        observation.as_ref(),
                        optional_pvp,
                        read_only_gate.as_ref(),
                        None,
                        None,
                    ))
                    .await
                }
                Err(disposition) => disposition,
            };
            let Some((runtime, states)) = guards.as_mut() else {
                return entered;
            };
            let unresolved =
                self.states
                    .quest_callbacks
                    .records
                    .lock()
                    .map_or(true, |mut records| {
                        if let Some(pending) = records.get_mut(&session) {
                            pending.origin = Some(origin);
                            true
                        } else {
                            false
                        }
                    });
            if result != UseDisposition::Committed && !unresolved {
                self.refuse_quest_step(
                    optional_pvp,
                    runtime,
                    states,
                    actor,
                    session,
                    map,
                    callback,
                    entered,
                    origin,
                );
            }
            runtime.read_actor_position(actor).unwrap_or(entered)
        };
        match tokio::time::timeout_at(deadline, operation).await {
            Ok(snapshot) => snapshot,
            Err(_) if callback.is_read_only() => {
                self.rollback_quest_step(actor, session, map, callback, entered, origin)
                    .await
            }
            Err(_) => {
                if let Ok(mut records) = self.states.quest_callbacks.records.lock() {
                    let pending = records.entry(session).or_insert(Pending {
                        actor,
                        session,
                        expected: entered,
                        callback: callback.key.clone(),
                        key: *key,
                        revision: map.facts.object_revision(*key),
                        cause,
                        origin: Some(origin),
                        context: None,
                        paid_arrivals: Vec::new(),
                        flame: None,
                        ingress_settled: false,
                        participants: Vec::new(),
                        projection_target: None,
                        flame_effect_sent: false,
                        selected_tool: None,
                    });
                    pending.origin = Some(origin);
                }
                entered
            }
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "binds the exact actor, session, movement snapshots and failed callback"
    )]
    async fn rollback_quest_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        entered: MovementPositionSnapshot,
        origin: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        let (mut runtime, mut states) = self.lock_callback_guards().await;
        if runtime.player_control_facts(actor, session).is_ok()
            && runtime.read_actor_position(actor) == Ok(entered)
        {
            self.refuse_quest_step(
                None,
                &mut runtime,
                &mut states,
                actor,
                session,
                map,
                callback,
                entered,
                origin,
            );
        }
        runtime.read_actor_position(actor).unwrap_or(entered)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "binds the exact actor, session, movement snapshots and source trigger"
    )]
    fn refuse_quest_step(
        &self,
        optional_pvp: Option<bool>,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        entered: MovementPositionSnapshot,
        origin: MovementPositionSnapshot,
    ) {
        if !callback.return_on_failure {
            return;
        }
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let blocked = super::pvp_live::pz_blocked(states, actor, session, self.owner_now());
        if let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport_facing(
            runtime,
            self.movement_cells(),
            actor,
            session,
            entered,
            callback.failure_teleport.map_or(origin.position(), local),
            blocked,
            &closed,
            None,
        ) && let Ok(snapshot) = runtime.commit_source_step(proof)
        {
            states.stage_special_movement_step(
                runtime,
                map,
                actor,
                session,
                self.owner_now().get(),
                Some(entered.position()),
                snapshot.position(),
                optional_pvp,
            );
            states.stage_underwater_step(
                runtime,
                actor,
                session,
                self.owner_now().get(),
                underwater(map, entered.position()),
                underwater(map, snapshot.position()),
            );
            self.punish_carrot_refusal(runtime, states, actor, session, entered, snapshot);
            if let Some(effect) = callback.effects.teleport_effect.or_else(|| {
                oteryn_gameplay_quests::quest::shared_switches::gate(&callback.key)
                    .map(|gate| gate.effect)
            }) && let Some(at) = native(snapshot.position())
            {
                let _ = map.facts.emit_movement_effect(at, effect);
            }
        }
        if let Some(message) = &callback.failure_message {
            self.states.chat.push_quest_feedback(session, message);
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A conditional source callback owns the route only when its full native binding
/// matches and callback selection at this tile is unambiguous. Ordinary step proofs
/// still enforce entry collision, doors, pacing, occupancy and protection-zone rules.
pub(super) struct CallbackPortals<'a> {
    pub inner: &'a dyn crate::map::floor::CellBlocker,
    pub map: Option<&'a BundleMap>,
    pub callbacks: &'a [Callback],
}
impl crate::map::floor::CellBlocker for CallbackPortals<'_> {
    fn blocks(&self, cell: crate::content::LogicalCell) -> bool {
        self.inner.blocks(cell)
    }
    fn owns_native_transition(&self, cell: crate::content::LogicalCell) -> bool {
        let Some(map) = self.map else {
            return false;
        };
        let Some(at) = native(MovementLocalPosition {
            x: cell.x,
            y: cell.y,
            floor: match i16::try_from(cell.z) {
                Ok(z) => z,
                Err(_) => return false,
            },
        }) else {
            return false;
        };
        let guarded = |c: &Callback| {
            c.trigger == Trigger::StepIn
                && c.actor_kind == world_callbacks::ActorKind::Player
                && c.teleport.is_some()
                && c.return_on_failure
                && (c.has_state() || !c.world_requirements.is_empty())
                && c.selector
                    .native
                    .as_ref()
                    .is_some_and(|source| tile(source.position) == at)
        };
        if !self.callbacks.iter().any(|c| {
            guarded(c)
                || oteryn_gameplay_quests::quest::carried_step::native_binding(&c.key)
                    .is_some_and(|b| tile(b.position) == at)
        }) {
            return false;
        }
        let Some(base) = map.overlay.base().tile(at.x, at.y, at.floor) else {
            return false;
        };
        let mut selected = None;
        for (ordinal, (&id, _)) in base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
        {
            let Ok(ordinal) = u8::try_from(ordinal) else {
                return false;
            };
            let Some(key) = placement_key(at, ordinal) else {
                return false;
            };
            if map.facts.base_hidden(key) {
                continue;
            }
            for callback in self.callbacks.iter().filter(|c| {
                c.actor_kind == world_callbacks::ActorKind::Player
                    && c.trigger == Trigger::StepIn
                    && matches(map, c, key)
            }) {
                if selected.replace((callback, key, ordinal, id)).is_some() {
                    return false;
                }
            }
        }
        selected.is_some_and(|(c, key, ordinal, id)| {
            (guarded(c)
                && c.selector.native.as_ref().is_some_and(|source| {
                    source.ordinal == ordinal
                        && map
                            .facts
                            .quest_callback_native_object(key, id, source.appearance)
                }))
                || oteryn_gameplay_quests::quest::carried_step::native_binding(&c.key).is_some_and(
                    |b| {
                        map.facts.quest_callback_native_teleport(
                            key,
                            b.appearance,
                            tile(b.raw_destination),
                        )
                    },
                )
        })
    }
}

fn world_requirements_match(map: &BundleMap, callback: &Callback) -> bool {
    callback.world_requirements.iter().all(|requirement| {
        // Closed Ground-only slots have no static anchor; the refreshed Ground
        // owner and fenced SQL read-set prove their portable roots separately.
        if requirement.appearance == 0 {
            let position = tile(requirement.position);
            return (requirement.ground_stack.is_some()
                || (requirement.ground.is_some() && callback.has_personal_state()))
                && !map.facts.house_tile(position)
                && map
                    .overlay
                    .base()
                    .tile(position.x, position.y, position.floor)
                    .is_some();
        }
        let position = tile(requirement.position);
        !map.facts.house_tile(position)
            && map
                .overlay
                .base()
                .tile(position.x, position.y, position.floor)
                .is_some_and(|base| {
                    base.ids()
                        .iter()
                        .zip(base.depths())
                        .filter(|(_, depth)| **depth == 0)
                        .enumerate()
                        .any(|(ordinal, (id, _))| {
                            let Ok(ordinal) = u8::try_from(ordinal) else {
                                return false;
                            };
                            let Some(key) = placement_key(position, ordinal) else {
                                return false;
                            };
                            !map.facts.base_hidden(key)
                                && map.facts.base_entry(position, ordinal, *id).is_some_and(
                                    |entry| {
                                        !entry.pickupable
                                            && entry.appearance_id == requirement.appearance
                                    },
                                )
                        })
                })
    })
}

/// Literal fixed external reset under the retained origin scope owner. This is
/// source-event composition, not an invented USE command or a Character grant.
pub(in crate::gameplay_transport) fn prepare_bak_public_reset(
    runtime: &crate::foundation::ChannelRuntimeV1,
    map: &crate::map::boot::BundleMap,
    prepared: crate::durability::quest_state::rotten_flame::PreparedBakEssences,
) -> Option<crate::durability::quest_state::rotten_flame::phase::BakPhasePublication> {
    rotten_flame::prepare_public_reset(runtime, map, prepared)
}

#[cfg(test)]
mod native_selector_tests {
    use super::*;

    #[test]
    fn native_selector_mismatch_never_falls_back_to_empty_legacy_selector()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::map::boot::BundleCollisionIndexExt;
        let world = crate::map::boot::tests::door_world()?;
        let mut world_bytes = [1; 16];
        world_bytes[6] = 0x70;
        world_bytes[8] = 0x80;
        let world_id = crate::foundation::WorldId::decode(&world_bytes)?;
        let room = crate::content::qualify_native_entry_room(world_id)?;
        let cells = world.movement_cells(room.movement_cells())?;
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            cells.index()
        else {
            return Err("bundle index".into());
        };
        let map = index.map();
        let position = crate::map::boot::tests::DOOR_AT;
        let key = crate::map::view::placement_key(position, 1).ok_or("source key")?;
        let base = map
            .overlay
            .base()
            .tile(position.x, position.y, position.floor)
            .ok_or("source tile")?;
        let id = *base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(1)
            .ok_or("source object")?
            .0;
        let appearance = map
            .facts
            .base_entry(position, 1, id)
            .ok_or("source appearance")?
            .appearance_id;
        let mut callback = world_callbacks::load()?
            .into_iter()
            .next()
            .ok_or("callback")?;
        callback.key = "oteryn:quest-callback/native-selector-regression".into();
        callback.selector.position = None;
        callback.selector.unique = None;
        callback.selector.action = None;
        callback.selector.appearance = None;
        callback.selector.native = Some(world_callbacks::NativeSelector {
            position: Position {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
            ordinal: 1,
            appearance,
        });
        assert!(
            matches(map, &callback, key),
            "real bare source binds exactly"
        );
        callback
            .selector
            .native
            .as_mut()
            .ok_or("selector")?
            .position
            .x += 1;
        assert!(
            !matches(map, &callback, key),
            "wrong native position must not use all-None legacy fallback"
        );
        callback.selector.appearance = Some(appearance);
        assert!(
            !matches(map, &callback, key),
            "appearance alone cannot authorize native mismatch fallback"
        );
        callback.selector.position = Some(Position {
            x: position.x,
            y: position.y,
            floor: position.floor,
        });
        assert!(
            matches(map, &callback, key),
            "retained exact legacy source position and door appearance remain valid"
        );
        callback.selector.position = None;
        callback.selector.appearance = None;

        callback
            .selector
            .native
            .as_mut()
            .ok_or("selector")?
            .position
            .x -= 1;
        callback
            .selector
            .native
            .as_mut()
            .ok_or("selector")?
            .appearance = appearance.checked_add(1).ok_or("appearance")?;
        assert!(
            !matches(map, &callback, key),
            "wrong native appearance must not use legacy fallback"
        );
        callback
            .selector
            .native
            .as_mut()
            .ok_or("selector")?
            .appearance = appearance;
        callback.selector.native.as_mut().ok_or("selector")?.ordinal = 0;
        assert!(
            !matches(map, &callback, key),
            "wrong native stack ordinal must not use legacy fallback"
        );
        Ok(())
    }
}
