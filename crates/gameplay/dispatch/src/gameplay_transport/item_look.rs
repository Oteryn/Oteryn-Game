//! ITEM-LOOK-1 examination called by the connection's capability-28 LOOK arm.
//!
//! The composed authority refreshes durable item facts and current runtime visibility
//! before examination. Handles alone never establish an item's continued availability.

use super::ComposedFreshAdmission;
use super::item_ref_admission::pinned_index;
use super::item_view::{ItemKey, SessionItemView, ViewItem};
use super::world_spatial::ActorPosition;
use crate::content::item_ref::{ITEM_FAMILY, ItemDefinitionIndex};
use crate::content::{ActiveGeneration, native_gameplay::QualifiedItemPolicy};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, VisibleRuntimeCorpse};
use crate::map::view::{self, MapEntryKey, MapFacts};
use crate::movement::interest::{VisibilityPosition, VisibilitySettings};
use crate::spell::Vocation;
use oteryn_protocol_oteryn::item_look::{
    ItemLookRefusal, ItemLookResult, MAX_ITEM_LOOK_TEXT_BYTES,
};
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::num::NonZeroU32;
use tokio::time::{Duration, Instant};

// The examine text is mounted here, not in `interaction/mod.rs`: the test targets that recompile
// `interaction/mod.rs` carry no `crate::content` (the `item_view::corpse_open` pattern).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemLookOutcome {
    Text(String),
    /// The same refusal for hidden, missing, stale, mismatched or unadmitted targets.
    Unavailable,
}

/// Fresh facts from the item owner; `item.key`, count and definition must belong to
/// the same observation. No instance identities or definition keys go on the wire.
pub struct ObservedLookItem<'a> {
    pub item: ViewItem,
    pub definition: &'a TypedDefinitionRef,
}

pub fn examine_observed_item(
    runtime: &ChannelRuntimeV1,
    generation: Option<&ActiveGeneration>,
    views: &SessionItemView,
    target: ItemHandle,
    observed: Option<ObservedLookItem<'_>>,
) -> ItemLookOutcome {
    // A surviving content pin does not override locally known owner retirement.
    // This denial is not proof of current DB/session authority or fresh visibility.
    if runtime.owner_fence().is_err() {
        return ItemLookOutcome::Unavailable;
    }
    let Some(key) = views.resolve(target) else {
        return ItemLookOutcome::Unavailable;
    };
    let Some(observed) = observed.filter(|observed| observed.item.key == key) else {
        return ItemLookOutcome::Unavailable;
    };
    let Some(generation) = generation else {
        return ItemLookOutcome::Unavailable;
    };
    let pin = runtime.content_pin();
    if pin.world_id() != generation.identity().world_id()
        || pin.activation_sequence() != generation.activation_sequence()
    {
        return ItemLookOutcome::Unavailable;
    }
    let Some(index) = pinned_index(runtime, Some(generation)) else {
        return ItemLookOutcome::Unavailable;
    };
    let definition = observed.definition;
    if index.definition_ref(
        &definition.family,
        &definition.production_key,
        &definition.revision_ref,
    ) != Some(observed.item.item_definition_ref)
    {
        return ItemLookOutcome::Unavailable;
    }
    let Some(policy) = generation.native_gameplay().and_then(|native| {
        native.item_policy(&definition.production_key, &definition.revision_ref)
    }) else {
        return ItemLookOutcome::Unavailable;
    };
    examine_qualified_item(policy, observed)
}

fn examine_qualified_item(
    policy: QualifiedItemPolicy<'_>,
    observed: ObservedLookItem<'_>,
) -> ItemLookOutcome {
    let record = policy.record();
    let expected = observed.definition;
    let actual = &record.production_definition;
    if !record.admission.materializable
        || actual.family != expected.family
        || actual.production_key != expected.production_key
        || actual.revision_ref != expected.revision_ref
    {
        return ItemLookOutcome::Unavailable;
    }
    oteryn_gameplay_items::look::item_text(&record.semantics, observed.item.count)
        .map(ItemLookOutcome::Text)
        .unwrap_or(ItemLookOutcome::Unavailable)
}

#[cfg(test)]
#[path = "item_look_tests.rs"]
mod tests;

/// CP D1143 / ITEMLOOK1-RL-01, per GameSession, including resume and transfer.
pub const ITEM_LOOK_COMMANDS_PER_SECOND: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookWindow([Option<Instant>; ITEM_LOOK_COMMANDS_PER_SECOND]);

impl LookWindow {
    pub const EMPTY: Self = Self([None; ITEM_LOOK_COMMANDS_PER_SECOND]);

    pub fn admit(&mut self, now: Instant) -> bool {
        let free = self.0.iter_mut().find(|slot| {
            !matches!(slot, Some(at) if now.saturating_duration_since(*at) < Duration::from_secs(1))
        });
        let Some(slot) = free else {
            return false;
        };
        *slot = Some(now);
        true
    }
}

/// Both positions use legacy z for runtime actors/corpses, or both use native -z for map
/// entries. `bundle_map_position` converts the map observer; never pair it with a runtime z.
fn in_view(actor: ActorPosition, target: ActorPosition) -> bool {
    match (
        VisibilityPosition::new(actor.x, actor.y, actor.floor),
        VisibilityPosition::new(target.x, target.y, target.floor),
    ) {
        (Ok(actor), Ok(target)) => VisibilitySettings::REFERENCE.can_see(actor, target),
        _ => false,
    }
}

pub(super) enum Looked<'a> {
    Creature {
        name: &'a str,
    },
    Player {
        name: &'a str,
        level: u32,
        vocation: Option<&'a str>,
    },
    Yourself {
        level: u32,
        vocation: Option<&'a str>,
    },
}

pub(super) fn look_text(looked: Looked<'_>) -> ItemLookResult {
    let (name, details) = match looked {
        Looked::Creature { name } => (name, None),
        Looked::Player {
            name,
            level,
            vocation,
        } => (name, Some((level, vocation))),
        Looked::Yourself { level, vocation } => ("yourself", Some((level, vocation))),
    };
    if name.trim().is_empty() {
        return ItemLookResult::Refused(ItemLookRefusal::NotAvailable);
    }
    let mut text = match details {
        None => format!("You see {name}."),
        Some((level, None)) => format!("You see {name} (Level {level})."),
        Some((level, Some(vocation))) => format!("You see {name} (Level {level}, {vocation})."),
    };
    let mut end = text.len().min(MAX_ITEM_LOOK_TEXT_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    ItemLookResult::Text(text)
}

pub(super) fn vocation_name(vocation: Vocation) -> &'static str {
    match vocation {
        Vocation::Druid => "druid",
        Vocation::ElderDruid => "elder druid",
        Vocation::Sorcerer => "sorcerer",
        Vocation::MasterSorcerer => "master sorcerer",
        Vocation::Knight => "knight",
        Vocation::EliteKnight => "elite knight",
        Vocation::Paladin => "paladin",
        Vocation::RoyalPaladin => "royal paladin",
        Vocation::Monk => "monk",
        Vocation::ExaltedMonk => "exalted monk",
    }
}

pub(super) fn typed_definition(
    index: &ItemDefinitionIndex,
    reference: NonZeroU32,
) -> Option<TypedDefinitionRef> {
    let (key, revision) = index.definition_at(reference)?;
    Some(TypedDefinitionRef {
        family: ITEM_FAMILY.into(),
        production_key: key.into(),
        revision_ref: revision.into(),
    })
}

/// The generation identity is immutable for a runtime's lifetime; copy only the binding
/// needed to detect a runtime replacement while durable owner facts are being read.
fn content_generation(runtime: &ChannelRuntimeV1) -> (u64, [u8; 32]) {
    (
        runtime.content_pin().activation_sequence(),
        runtime.content_pin().server_artifact_digest(),
    )
}

/// Read current position under the runtime lock, never from a cached session viewport.
fn actor_position(runtime: &mut ChannelRuntimeV1, actor: ExactActorRef) -> Option<ActorPosition> {
    let position = runtime
        .borrow_movement_position()
        .read(actor)
        .ok()?
        .position();
    Some(ActorPosition {
        x: position.x,
        y: position.y,
        floor: position.floor,
    })
}

/// The corpse binding and spatial predicate must survive the durable contents read.
fn corpse_for_look(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    identity: [u8; 16],
    entry: bool,
) -> Result<VisibleRuntimeCorpse, ItemLookRefusal> {
    let position = actor_position(runtime, actor).ok_or(ItemLookRefusal::NotAvailable)?;
    let refusal = if entry {
        ItemLookRefusal::NotReachable
    } else {
        ItemLookRefusal::NotVisible
    };
    let corpse = runtime.bound_corpse(identity).ok_or(refusal)?;
    let target = ActorPosition {
        x: corpse.position.x,
        y: corpse.position.y,
        floor: corpse.position.floor,
    };
    if if entry {
        super::item_view::corpse_open::within_reach(position, target)
    } else {
        in_view(position, target)
    } {
        Ok(corpse)
    } else {
        Err(refusal)
    }
}

pub(super) async fn look_item(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    fence: Option<CurrentCharacterItemFence>,
    views: &SessionItemView,
    target: ItemHandle,
) -> ItemLookResult {
    match observe_look_item(admission, actor, session, fence, views, target).await {
        Ok(result) => result,
        Err(refusal) => ItemLookResult::Refused(refusal),
    }
}

async fn observe_look_item(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    fence: Option<CurrentCharacterItemFence>,
    views: &SessionItemView,
    target: ItemHandle,
) -> Result<ItemLookResult, ItemLookRefusal> {
    use super::item_ref_admission::definition_ref;
    let unavailable = ItemLookRefusal::NotAvailable;
    let key = views.resolve(target).ok_or(unavailable)?;
    let pin = content_generation(&*admission.runtime.lock().await);
    let corpse_key = views
        .open_corpse_of(&key)
        .or_else(|| matches!(key, ItemKey::Entity(_)).then_some(key));
    let corpse = if let Some(ItemKey::Entity(identity)) = corpse_key {
        Some(corpse_for_look(
            &mut *admission.runtime.lock().await,
            actor,
            identity,
            key != ItemKey::Entity(identity),
        )?)
    } else {
        None
    };
    let (definition, count) = match key {
        ItemKey::Entity(_) => {
            let corpse = corpse.as_ref().ok_or(unavailable)?;
            (
                TypedDefinitionRef {
                    family: corpse.item.family.clone(),
                    production_key: corpse.item.production_key.clone(),
                    revision_ref: corpse.item.revision_ref.clone(),
                },
                1,
            )
        }
        ItemKey::Instance(instance) => {
            if let Some(slot) = views.equipped_slot(&key) {
                let equipment = super::equipment_move::read_current_equipment(admission, session)
                    .await
                    .ok_or(unavailable)?;
                let item = equipment
                    .items
                    .into_iter()
                    .find(|item| {
                        item.item_instance_id == instance
                            && oteryn_protocol_oteryn::item_view::wire_slot(item.slot) == Some(slot)
                    })
                    .ok_or(unavailable)?;
                (item.definition, item.quantity)
            } else if let Some(corpse) = &corpse {
                let contents = admission
                    .root
                    .read_corpse_contents(admission.character, corpse.item.item_instance_id)
                    .await
                    .ok()
                    .flatten()
                    .ok_or(unavailable)?;
                let entry = contents
                    .into_iter()
                    .find(|entry| entry.item.item_instance_id == instance)
                    .ok_or(unavailable)?;
                (entry.item.definition, entry.item.quantity)
            } else {
                // This fenced owner read admits only the Character's carried tree, including
                // the main backpack. A handle may instead name a durable Ground item, which
                // needs no carried-item fence: without one the look goes straight to the map.
                let read = match fence {
                    Some(fence) => admission
                        .root
                        .read_item_loop_container(admission.character, fence.character_id, instance)
                        .await
                        .map_err(|_| unavailable)?,
                    None => None,
                };
                match read {
                    Some(read) => (read.container.definition, read.container.quantity),
                    None => observe_map_look(admission, actor, key).await?,
                }
            }
        }
        ItemKey::MapBase { .. } | ItemKey::MapAdded(_) => {
            observe_map_look(admission, actor, key).await?
        }
    };
    let mut runtime = admission.runtime.lock().await;
    if content_generation(&runtime) != pin {
        return Err(unavailable);
    }
    actor_position(&mut runtime, actor).ok_or(unavailable)?;
    if let (Some(ItemKey::Entity(identity)), Some(previous)) = (corpse_key, corpse) {
        let current = corpse_for_look(
            &mut runtime,
            actor,
            identity,
            key != ItemKey::Entity(identity),
        )?;
        if current.item != previous.item {
            return Err(unavailable);
        }
    }
    let index = pinned_index(&runtime, admission.active_generation).ok_or(unavailable)?;
    let observed = ObservedLookItem {
        item: ViewItem {
            key,
            item_definition_ref: definition_ref(index, &definition).ok_or(unavailable)?,
            count: NonZeroU32::new(count).ok_or(unavailable)?,
            sub_type: 0,
        },
        definition: &definition,
    };
    match examine_observed_item(
        &runtime,
        admission.active_generation,
        views,
        target,
        Some(observed),
    ) {
        ItemLookOutcome::Text(text) => Ok(ItemLookResult::Text(text)),
        ItemLookOutcome::Unavailable => Err(unavailable),
    }
}

/// Resolve a current map entry using native floors for both observer and target.
pub(super) fn map_look_target<F: MapFacts>(
    source: &super::world_map::MapViewSource<'_, F>,
    at: ActorPosition,
    key: ItemKey,
    index: &ItemDefinitionIndex,
) -> Result<(TypedDefinitionRef, NonZeroU32), ItemLookRefusal> {
    use super::world_map::{MapHandleTarget, resolve_map_handle};
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    let unavailable = ItemLookRefusal::NotAvailable;
    let (pos, entry_key) = match resolve_map_handle(source, key).map_err(|_| unavailable)? {
        MapHandleTarget::Base {
            pos, placement_key, ..
        } => (pos, MapEntryKey::Base(placement_key)),
        MapHandleTarget::Added { pos, .. } => {
            let entry = source
                .overlay
                .tile(pos)
                .and_then(|tile| {
                    tile.added()
                        .iter()
                        .find(|entry| ItemKey::MapAdded(entry.id().get()) == key)
                })
                .ok_or(unavailable)?;
            (pos, MapEntryKey::Added(entry.id()))
        }
        MapHandleTarget::Ground {
            pos,
            item_instance_id,
        } => (pos, MapEntryKey::Ground(item_instance_id)),
    };
    let target = ActorPosition {
        x: i32::from(pos.x),
        y: i32::from(pos.y),
        floor: i16::from(pos.floor),
    };
    if !in_view(at, target) {
        return Err(ItemLookRefusal::NotVisible);
    }
    let tile = view::compose_tile(source.overlay, source.facts, pos)
        .map_err(|_| unavailable)?
        .ok_or(unavailable)?;
    let entry = tile
        .entries
        .iter()
        .find(|entry| entry.source.key() == entry_key)
        .ok_or(unavailable)?;
    let MapDefinition::Item(reference) = entry.facts.definition else {
        return Err(unavailable);
    };
    Ok((
        typed_definition(index, reference).ok_or(unavailable)?,
        NonZeroU32::new(entry.facts.count).ok_or(unavailable)?,
    ))
}

async fn observe_map_look(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    key: ItemKey,
) -> Result<(TypedDefinitionRef, u32), ItemLookRefusal> {
    use super::world_map::MapViewSource;
    let unavailable = ItemLookRefusal::NotAvailable;
    let (at, index, pin) = {
        let mut runtime = admission.runtime.lock().await;
        let at = super::bundle_map_position(&mut runtime, admission.movement_cells, actor)
            .ok_or(unavailable)?;
        let index = pinned_index(&runtime, admission.active_generation).ok_or(unavailable)?;
        (at, index, content_generation(&runtime))
    };
    let ground = super::loop_move::loop_ground(admission, at.map.overlay.base(), Some(index)).await;
    // Re-read the actor's native position and pin after the durable Ground observation.
    let mut runtime = admission.runtime.lock().await;
    if content_generation(&runtime) != pin
        || !pinned_index(&runtime, admission.active_generation)
            .is_some_and(|current| std::ptr::eq(current, index))
    {
        return Err(unavailable);
    }
    let current = super::bundle_map_position(&mut runtime, admission.movement_cells, actor)
        .ok_or(unavailable)?;
    let facts = at.map.facts.pinned();
    let (definition, count) = match ground.as_deref() {
        Some(ground) => {
            let facts = ground.facts(&facts);
            let source = MapViewSource {
                overlay: ground.overlay(),
                facts: &facts,
                content_generation: at.content_generation,
                reset_epoch: 0,
            };
            map_look_target(&source, current.at, key, index)?
        }
        None => {
            let source = MapViewSource {
                overlay: &at.map.overlay,
                facts: &facts,
                content_generation: at.content_generation,
                reset_epoch: 0,
            };
            map_look_target(&source, current.at, key, index)?
        }
    };
    Ok((definition, count.get()))
}

/// Owned runtime facts; no borrowed runtime state survives the following durable read.
pub(super) enum RuntimeLook {
    Text(ItemLookResult),
    Player {
        actor: ExactActorRef,
        session: GameSessionId,
        yourself: bool,
    },
}

/// Current exact actor and REFERENCE view admission, entirely under the runtime lock.
pub(super) fn runtime_look_creature(
    runtime: &mut ChannelRuntimeV1,
    npcs: Option<&super::npc_talk::NpcTalkBook>,
    actor: ExactActorRef,
    target: EntityRef,
) -> Result<RuntimeLook, ItemLookRefusal> {
    let position = actor_position(runtime, actor).ok_or(ItemLookRefusal::NotAvailable)?;
    let visible = runtime.visible_entities();
    let found = visible
        .players
        .iter()
        .chain(&visible.creatures)
        .chain(&visible.npcs)
        .find(|entry| {
            entry.actor.placement_identity() == target.identity
                && entry.generation == target.generation
        })
        .ok_or(ItemLookRefusal::NotVisible)?;
    let target_position = ActorPosition {
        x: found.position.x,
        y: found.position.y,
        floor: found.position.floor,
    };
    if !in_view(position, target_position) {
        return Err(ItemLookRefusal::NotVisible);
    }
    if visible
        .creatures
        .iter()
        .any(|entry| entry.actor == found.actor)
    {
        let snapshot = runtime
            .companion_snapshot(found.actor)
            .map_err(|_| ItemLookRefusal::NotAvailable)?;
        return Ok(RuntimeLook::Text(look_text(Looked::Creature {
            name: &snapshot.state.policy.display_name,
        })));
    }
    if visible.npcs.iter().any(|entry| entry.actor == found.actor) {
        let npc = runtime
            .npc_key(found.actor)
            .and_then(|key| npcs?.get(key))
            .ok_or(ItemLookRefusal::NotAvailable)?;
        return Ok(RuntimeLook::Text(look_text(Looked::Creature {
            name: &npc.name,
        })));
    }
    // There is no actor -> session accessor. The bounded occupied-slot melee census includes
    // visible players whose control was lost; the normal census excludes those players.
    // `positioned_player_for_session` is the inverse lookup and requires an already known session.
    let session = runtime
        .positioned_melee_census()
        .ok()
        .and_then(|census| {
            census
                .into_iter()
                .find(|(actor, _, _)| *actor == found.actor)
                .and_then(|(_, _, session)| session)
        })
        .ok_or(ItemLookRefusal::NotAvailable)?;
    Ok(RuntimeLook::Player {
        actor: found.actor,
        session,
        yourself: found.actor == actor,
    })
}

pub(super) async fn look_creature(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    target: EntityRef,
) -> ItemLookResult {
    let (session, yourself, vocation) = {
        let mut runtime = admission.runtime.lock().await;
        match runtime_look_creature(&mut runtime, admission.chat.npc_talk(), actor, target) {
            Err(refusal) => return ItemLookResult::Refused(refusal),
            Ok(RuntimeLook::Text(text)) => return text,
            Ok(RuntimeLook::Player {
                actor,
                session,
                yourself,
            }) => {
                // Lock order runtime -> spell_states. Vocation is optional, so never await a
                // contended spell-state lock while holding the runtime.
                let vocation = admission.spell_states.try_lock().ok().and_then(|states| {
                    states
                        .get(&runtime, actor, session)
                        .map(|state| vocation_name(state.character_facts().vocation))
                });
                (session, yourself, vocation)
            }
        }
    };
    // Each of current_session_at, chat-name and progression reads runs in the root's bounded
    // semantic pass (including holder acquisition and PostgreSQL statement/lock deadlines).
    let Some((name, _, level)) = admission.chat_speaker_name(session).await else {
        return ItemLookResult::Refused(ItemLookRefusal::NotAvailable);
    };
    look_text(if yourself {
        Looked::Yourself { level, vocation }
    } else {
        Looked::Player {
            name: &name,
            level,
            vocation,
        }
    })
}
