//! ITEM-LOOP-1 (CP D1438): the production item loop of command 9 and its views.
//!
//! - **Moves.** [`move_loop_item`] decides the Channel half of one [`LoopMove`] under the spell
//!   lane and the runtime lock: a living actor with no pending spell commit, the Channel content
//!   pin of the active generation and, for a bundle World, the actor's native position. A pickup
//!   names the Ground cell the map view shows the item on and must be in reach (Chebyshev
//!   distance <= 1 on the actor's floor), as must a drop's cell, which must also be a base tile.
//!   The loop owner (`durability::item_loop_move`) decides the rest in one transaction.
//! - **Ground view.** The loop Ground items of the Channel are read from the item owner into a
//!   [`LoopGroundCache`], which every committed drop or pickup invalidates; the map view
//!   (domain 17) shows them over the bundle map, each with its definition's facts. An item whose
//!   `map_revision` is not the bundle's, or whose definition the pinned index does not name, is
//!   not shown. When the read fails the view shows the bundle map alone.
//! - **Containers (capability 14).** [`observe_container`] reads a container of the carried tree
//!   with its direct entries; a container is carried, so it is always in reach.

use super::ComposedFreshAdmission;
use super::connection::UseCommand;
use super::container_view::ContainerObservation;
use super::equipment_move::native_for_move;
use super::item_move::{LoopMove, LoopMoveRefusal, LoopMoveReplay, LoopMoveResult, LoopTarget};
use super::item_ref_admission::{
    NativeItemWeights, definition_ref, generation_item_facts, pinned_index,
};
use super::item_view::{ItemKey, ViewItem};
use crate::content::item_ref::ItemDefinitionIndex;
use crate::durability::item_loop_move::{
    ItemLoopContent, LoopDestination, LoopGround, LoopMoveRequest,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{CurrentCharacterItemFence, ItemDefinitionFacts};
use crate::foundation::{ChannelId, CommandId, CommandRef, ExactActorRef, WorldId};
use crate::map::WorldBase;
use crate::map::overlay::{AddedItem, ChannelOverlay, EntryId, TilePos, map_revision};
use crate::map::view::{self, EntryFacts, MapFacts};
use oteryn_protocol_oteryn::container_tree::MAX_CONTAINER_VIEW_ENTRIES;
use oteryn_protocol_oteryn::item_view::WorldTilePosition;
use oteryn_protocol_oteryn::world_map::MapDefinition;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

/// D1211: the loop decides every touched definition from this exact admitted native generation.
impl ItemLoopContent for NativeItemWeights<'_> {
    fn loop_content_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }

    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        generation_item_facts(self.0, definition).ok()
    }
}

/// The loop Ground items of one Channel as a map overlay over the bundle base, with the facts
/// the map view shows each with.
#[derive(Debug)]
pub struct LoopGroundOverlay {
    overlay: ChannelOverlay,
    facts: HashMap<[u8; 16], EntryFacts>,
}

impl LoopGroundOverlay {
    /// An overlay of `items` over `base`, lowest stack ordinal first. An item that does not
    /// belong to this World, Channel and bundle, whose cell is not a base tile, or whose
    /// definition `index` does not name, is left out.
    pub fn build(
        base: Arc<WorldBase>,
        world: WorldId,
        channel: ChannelId,
        index: &ItemDefinitionIndex,
        mut items: Vec<crate::durability::item_loop_move::LoopGroundItem>,
    ) -> Self {
        items.sort_by_key(|item| item.stack_ordinal);
        let revision = map_revision(&base);
        let mut overlay = ChannelOverlay::new(base, world, channel);
        let mut facts = HashMap::new();
        for entry in items {
            let item = entry.item;
            if item.ground.map_revision != revision {
                continue;
            }
            let Some(reference) = definition_ref(index, &item.definition) else {
                continue;
            };
            let entry_facts = EntryFacts {
                definition: MapDefinition::Item(reference),
                terrain_kind: None,
                appearance_id: view::appearance_id(&item.definition.production_key, None),
                blocks_projectile: false,
                pickupable: true,
                bound: false,
                count: item.quantity.clamp(1, 100),
                sub_type: 0,
            };
            let id = item.item_instance_id;
            if overlay.add_ground(item).is_ok() {
                facts.insert(id, entry_facts);
            }
        }
        Self { overlay, facts }
    }

    pub fn overlay(&self) -> &ChannelOverlay {
        &self.overlay
    }

    /// Where a loop Ground item lies.
    pub fn ground_entry(&self, item: &[u8; 16]) -> Option<(TilePos, EntryId)> {
        self.overlay.ground_entry(item)
    }

    /// `inner` with the facts of these Ground items.
    pub fn facts<'a, F: MapFacts>(&'a self, inner: &'a F) -> LoopGroundFacts<'a, F> {
        LoopGroundFacts {
            inner,
            ground: &self.facts,
        }
    }
}

/// The bundle's map facts plus the facts of the loop Ground items.
pub struct LoopGroundFacts<'a, F> {
    inner: &'a F,
    ground: &'a HashMap<[u8; 16], EntryFacts>,
}

impl<F: MapFacts> MapFacts for LoopGroundFacts<'_, F> {
    fn base_entry(&self, pos: TilePos, ordinal: u8, id: u32) -> Option<EntryFacts> {
        self.inner.base_entry(pos, ordinal, id)
    }

    fn added_entry(&self, item: &AddedItem) -> Option<EntryFacts> {
        match item {
            AddedItem::Ground(ground) => self.ground.get(&ground.item_instance_id).copied(),
            AddedItem::Volatile { .. } => self.inner.added_entry(item),
        }
    }

    fn house_tile(&self, pos: TilePos) -> bool {
        self.inner.house_tile(pos)
    }

    fn object_revision(&self, placement_key: u64) -> u64 {
        self.inner.object_revision(placement_key)
    }
}

#[derive(Debug, Default)]
struct CacheState {
    epoch: u64,
    current: Option<Arc<LoopGroundOverlay>>,
}

/// The Channel's loop Ground overlay, read once after each change. A leaf lock, never held
/// across an await.
#[derive(Debug, Default)]
pub struct LoopGroundCache {
    state: Mutex<CacheState>,
}

impl LoopGroundCache {
    /// A drop or pickup may have changed the Ground: the next read rebuilds.
    pub fn invalidate(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.epoch = state.epoch.wrapping_add(1);
            state.current = None;
        }
    }

    fn cached(&self) -> (u64, Option<Arc<LoopGroundOverlay>>) {
        self.state
            .lock()
            .map(|state| (state.epoch, state.current.clone()))
            .unwrap_or((0, None))
    }

    fn store(&self, epoch: u64, built: &Arc<LoopGroundOverlay>) {
        if let Ok(mut state) = self.state.lock()
            && state.epoch == epoch
        {
            state.current = Some(built.clone());
        }
    }
}

/// The Channel's loop Ground overlay over the bundle `base`; `None` without a pinned index or
/// when the item owner cannot be read.
pub async fn loop_ground(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    base: &Arc<WorldBase>,
    index: Option<&ItemDefinitionIndex>,
) -> Option<Arc<LoopGroundOverlay>> {
    let (epoch, cached) = admission.loop_ground.cached();
    if let Some(cached) = cached {
        return Some(cached);
    }
    let index = index?;
    let items = admission
        .root
        .read_item_loop_ground(admission.world_id, admission.channel_id)
        .await
        .ok()?;
    let built = Arc::new(LoopGroundOverlay::build(
        base.clone(),
        admission.world_id,
        admission.channel_id,
        index,
        items,
    ));
    admission.loop_ground.store(epoch, &built);
    Some(built)
}

/// The loop item receipt of command 9's CommandRef. A session without an item fence has none.
pub(super) async fn committed_loop_move(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
) -> Option<LoopMoveReplay> {
    let Some(fence) = command.item_fence else {
        return Some(LoopMoveReplay::Absent);
    };
    let command_id = CommandId::new(command.command_id).ok()?;
    admission
        .root
        .read_item_loop_move_receipt(
            admission.character,
            fence.character_id,
            CommandRef::new(command.game_session_id, command_id),
        )
        .await
        .ok()
}

/// The 10-byte Ground cell codec of the item owner (x i32 BE, y i32 BE, floor i16 BE).
fn ground_cell(x: i32, y: i32, floor: i16) -> Vec<u8> {
    let mut cell = Vec::with_capacity(10);
    cell.extend_from_slice(&x.to_be_bytes());
    cell.extend_from_slice(&y.to_be_bytes());
    cell.extend_from_slice(&floor.to_be_bytes());
    cell
}

/// Same floor and Chebyshev distance <= 1 in the native frame.
fn in_reach(actor: (i32, i32, i16), cell: (i32, i32, i16)) -> bool {
    oteryn_gameplay_items::reach::within_reach(
        (actor.0, actor.1, i32::from(actor.2)),
        (cell.0, cell.1, i32::from(cell.2)),
    )
}

fn sha256_revision(digest: [u8; 32]) -> String {
    let mut revision = String::with_capacity(71);
    revision.push_str("sha256:");
    for byte in digest {
        revision.push_str(&format!("{byte:02x}"));
    }
    revision
}

/// One move of command 9 through the loop owner. A dead actor, a pending spell commit, another
/// Channel generation, a fixture World for a Ground move, a session without an item fence or a
/// source that is not a durable instance is refused before any durable write; a Ground cell out
/// of reach is `TooFar`; a durable failure is `Unknown`.
pub(super) async fn move_loop_item(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    request: LoopMove,
) -> LoopMoveResult {
    let refused = LoopMoveResult::Refused;
    // Lock order (SPELL-LOCK-2 §1.2): the lane before any Channel guard; the move is a key-33
    // writer. `None` is retryable with no side effect while the lane is fenced.
    let Some(permit) = admission.spell_lane_permit().await else {
        return LoopMoveResult::Unknown;
    };
    let ItemKey::Instance(item) = request.item else {
        return refused(LoopMoveRefusal::NotSupported);
    };
    let (native, pins, index) = {
        let mut runtime = admission.runtime.lock().await;
        let states = admission.spell_states.lock().await;
        let dead = states.is_dead(actor);
        let pending = states.has_pending_spell_commit(actor, command.game_session_id);
        drop(states);
        let native = native_for_move(&runtime, admission.active_generation, dead, pending);
        let index = pinned_index(&runtime, admission.active_generation);
        let pin = runtime.content_pin();
        let content_revision = sha256_revision(pin.server_artifact_digest());
        let placement_context = pin.frame_binding_digest().to_vec();
        let pins =
            super::bundle_map_position(&mut runtime, admission.movement_cells, actor).map(|at| {
                (
                    at.map.overlay.base().clone(),
                    (at.at.x, at.at.y, at.at.floor),
                    content_revision,
                    placement_context,
                )
            });
        (native, pins, index)
    };
    let Some(native) = native else {
        return refused(LoopMoveRefusal::Rejected);
    };
    let Some(fence) = command.item_fence else {
        return refused(LoopMoveRefusal::Rejected);
    };
    let Ok(command_id) = CommandId::new(command.command_id) else {
        return refused(LoopMoveRefusal::Rejected);
    };
    // ITEM-MOVE-2b: an item the map view shows on the Ground is a pickup from its cell.
    let ground = match &pins {
        Some((base, ..)) => loop_ground(admission, base, index).await,
        None => None,
    };
    let pickup = ground
        .as_ref()
        .and_then(|ground| ground.ground_entry(&item))
        .map(|(pos, _)| (i32::from(pos.x), i32::from(pos.y), i16::from(pos.floor)));
    let actor_at = pins.as_ref().map(|(_, at, ..)| *at);
    let pickup_cell = match (pickup, actor_at) {
        (Some(cell), Some(at)) if in_reach(at, cell) => Some(ground_cell(cell.0, cell.1, cell.2)),
        (Some(_), _) => return LoopMoveResult::TooFar,
        (None, _) => None,
    };
    let destination = match request.target {
        LoopTarget::MainBackpack => LoopDestination::MainBackpack,
        LoopTarget::Container(ItemKey::Instance(container)) => {
            LoopDestination::Container(container)
        }
        LoopTarget::Container(_) => return refused(LoopMoveRefusal::NotSupported),
        LoopTarget::Ground(to) => {
            let Some((base, at, content_revision, placement_context)) = &pins else {
                return refused(LoopMoveRefusal::NotSupported);
            };
            match drop_cell(base, *at, to) {
                Ok(spatial_position) => LoopDestination::Ground(LoopGround {
                    spatial_position,
                    map_revision: map_revision(base),
                    content_revision: content_revision.clone(),
                    placement_context: placement_context.clone(),
                }),
                Err(result) => return result,
            }
        }
    };
    let touches_ground = pickup_cell.is_some() || matches!(destination, LoopDestination::Ground(_));
    let committed = admission
        .root
        .commit_item_loop_move(
            &permit,
            admission.character,
            admission.holder,
            &fence,
            CommandRef::new(command.game_session_id, command_id),
            &NativeItemWeights(native),
            LoopMoveRequest {
                item,
                count: None,
                pickup_cell,
                destination,
            },
        )
        .await;
    drop(permit);
    if touches_ground && !matches!(committed, Ok(Err(_))) {
        admission.loop_ground.invalidate();
    }
    // ATTACK-2: a moved item may have been the swing weapon's container; the next swing reads
    // the equipment again.
    admission
        .attack
        .lock()
        .await
        .forget_weapon(actor, command.game_session_id);
    match committed {
        Ok(Ok(_receipt)) => LoopMoveResult::Committed,
        Ok(Err(refusal)) => LoopMoveResult::Refused(refusal),
        Err(_) => LoopMoveResult::Unknown,
    }
}

/// The 10-byte cell of a drop onto `to`: a base tile of the bundle in reach of the actor at
/// `at`; otherwise the refusal.
fn drop_cell(
    base: &WorldBase,
    at: (i32, i32, i16),
    to: WorldTilePosition,
) -> Result<Vec<u8>, LoopMoveResult> {
    if !in_reach(at, (to.x, to.y, to.floor)) {
        return Err(LoopMoveResult::TooFar);
    }
    let tile = match (
        u16::try_from(to.x),
        u16::try_from(to.y),
        i8::try_from(to.floor),
    ) {
        (Ok(x), Ok(y), Ok(floor)) => base.tile(x, y, floor),
        _ => None,
    };
    if tile.is_none() {
        return Err(LoopMoveResult::Refused(LoopMoveRefusal::Blocked));
    }
    Ok(ground_cell(to.x, to.y, to.floor))
}

/// BAGS-1: a container of the fenced Character's carried tree (the main backpack included) with
/// its direct entries. `None` for anything that is not an item instance, without an item fence
/// or a pinned index, or when the item owner cannot be read; an instance outside the carried
/// tree is `Gone`, a carried item that is no container `NotAContainer`.
pub(super) async fn observe_container(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    fence: Option<CurrentCharacterItemFence>,
    target: ItemKey,
) -> Option<ContainerObservation> {
    let ItemKey::Instance(container) = target else {
        return None;
    };
    let fence = fence?;
    let index = pinned_index(
        &*admission.runtime.lock().await,
        admission.active_generation,
    )?;
    let native = admission.active_generation?.native_gameplay()?;
    let Some(read) = admission
        .root
        .read_item_loop_container(admission.character, fence.character_id, container)
        .await
        .ok()?
    else {
        return Some(ContainerObservation::Gone);
    };
    let capacity = generation_item_facts(native, &read.container.definition)
        .ok()
        .and_then(|facts| facts.container_capacity)
        .and_then(|capacity| u8::try_from(capacity).ok())
        .filter(|capacity| (1..=MAX_CONTAINER_VIEW_ENTRIES).contains(&usize::from(*capacity)));
    let Some(capacity) = capacity else {
        return Some(ContainerObservation::NotAContainer);
    };
    let entries = read
        .entries
        .iter()
        .take(usize::from(capacity))
        .map(|entry| {
            Some(ViewItem {
                key: ItemKey::Instance(entry.item.item_instance_id),
                item_definition_ref: definition_ref(index, &entry.item.definition)?,
                count: NonZeroU32::new(entry.item.quantity)?,
                sub_type: 0,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ContainerObservation::Container {
        parent: read.parent.map(ItemKey::Instance),
        capacity,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ground_cell_codec_is_the_item_owners() {
        let cell = ground_cell(1000, 2000, -7);
        let decoded = crate::durability::spell_items_abi::decode_ground_cell(&cell).map_err(|_| ());
        assert_eq!(
            decoded.map(|cell| (cell.x, cell.y, cell.z)),
            Ok((1000, 2000, -7))
        );
    }

    #[test]
    fn reach_is_one_tile_on_the_actors_floor() {
        assert!(in_reach((10, 10, -7), (11, 9, -7)));
        assert!(in_reach((10, 10, -7), (10, 10, -7)));
        assert!(!in_reach((10, 10, -7), (12, 10, -7)));
        assert!(!in_reach((10, 10, -7), (10, 10, -6)));
    }

    #[test]
    fn the_content_revision_is_the_prefixed_digest() {
        let revision = sha256_revision([0xab; 32]);
        assert_eq!(revision.len(), 71);
        assert!(revision.starts_with("sha256:abab"));
    }
}
