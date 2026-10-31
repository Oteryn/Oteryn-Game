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
//!   not shown. A failed durable read fails the map view closed.
//! - **Containers (capability 14).** [`observe_container`] reads carried containers and proven
//!   Ground container trees within one tile on the actor's floor, under the Channel lane.

#[path = "loop_map_pickup.rs"]
mod map_pickup;

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
use sha2::{Digest, Sha256};
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
    fn loop_definition(&self, reference: u32) -> Option<TypedDefinitionRef> {
        let (key, revision) = self
            .0
            .item_index()?
            .definition_at(std::num::NonZeroU32::new(reference)?)?;
        Some(TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: revision.into(),
        })
    }
    fn is_quiver(&self, definition: &TypedDefinitionRef) -> bool {
        definition.family == "Item"
            && matches!(
                definition.production_key.as_str(),
                "oteryn:item.tibia.i35562"
                    | "oteryn:item.tibia.i35848"
                    | "oteryn:item.tibia.i35849"
                    | "oteryn:item.tibia.i35524"
                    | "oteryn:item.tibia.i36666"
                    | "oteryn:item.tibia.i39150"
                    | "oteryn:item.tibia.i39160"
                    | "oteryn:item.tibia.i45644"
            )
            && self.loop_facts(definition).is_some()
    }
    fn is_ammunition(&self, definition: &TypedDefinitionRef) -> bool {
        self.0
            .item_policy(&definition.production_key, &definition.revision_ref)
            .is_some_and(|p| {
                matches!(
                    p.record().authoring.ranged,
                    Some(crate::content::ProjectV2RangedWeapon::Ammunition { .. })
                )
            })
    }
    fn loot_category(
        &self,
        definition: &TypedDefinitionRef,
    ) -> oteryn_gameplay_items::quick_loot::LootCategory {
        use oteryn_gameplay_items::quick_loot::LootCategory;
        if crate::durability::item_loop_move::is_loot_coin(definition) {
            return LootCategory::Gold;
        }
        let Some(policy) = self
            .0
            .item_policy(&definition.production_key, &definition.revision_ref)
        else {
            return LootCategory::Other;
        };
        if let Some(taxonomy) = &policy.record().authoring.taxonomy {
            match taxonomy.primary.to_ascii_lowercase().as_str() {
                "creature products" => return LootCategory::CreatureProduct,
                "potions" | "food" | "runes" | "attack runes" | "healing runes"
                | "support runes" | "ammunition" => return LootCategory::Consumable,
                "armors"
                | "helmets"
                | "legs"
                | "boots"
                | "rings"
                | "amulets and necklaces"
                | "shields"
                | "spellbooks"
                | "quivers"
                | "sword weapons"
                | "axe weapons"
                | "club weapons"
                | "distance weapons"
                | "fist fighting weapons"
                | "wands"
                | "rods"
                | "armas de arremesso"
                | "clavas" => return LootCategory::Equipment,
                _ => {}
            }
        }
        use crate::content::ReferenceItemField;
        if matches!(
            policy.record().semantics.equipment,
            ReferenceItemField::Known(_)
        ) {
            return LootCategory::Equipment;
        }
        if self.is_ammunition(definition) {
            return LootCategory::Consumable;
        }
        LootCategory::Other
    }
}

/// The loop Ground items of one Channel as a map overlay over the bundle base, with the facts
/// the map view shows each with.
#[derive(Debug)]
pub struct LoopGroundOverlay {
    overlay: ChannelOverlay,
    facts: HashMap<[u8; 16], EntryFacts>,
    field_pin: ([u8; 32], [u8; 32], u64),
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
        field_pin: ([u8; 32], [u8; 32], u64),
        mut items: Vec<crate::durability::item_loop_move::LoopGroundItem>,
    ) -> Self {
        items.sort_by_key(|item| item.stack_ordinal);
        let revision = map_revision(&base);
        let field_revision = sha256_revision(Sha256::digest(revision.as_bytes()).into());
        let field_content = sha256_revision(field_pin.0);
        let mut overlay = ChannelOverlay::new(base, world, channel);
        let mut facts = HashMap::new();
        for entry in items {
            let mut item = entry.item;
            if entry.field_collision.is_some() {
                if item.ground.map_revision != field_revision
                    || item.ground.content_revision != field_content
                    || item.ground.native_room_placement_context.as_slice()
                        != field_pin.1.as_slice()
                    || item.runtime_scope_ownership_generation != field_pin.2
                {
                    continue;
                }
                item.ground.map_revision = revision.clone();
            } else if item.ground.map_revision != revision {
                continue;
            }
            let Some(reference) = definition_ref(index, &item.definition) else {
                continue;
            };
            let entry_facts = EntryFacts {
                definition: MapDefinition::Item(reference),
                terrain_kind: None,
                appearance_id: view::appearance_id(&item.definition.production_key, None),
                blocks_projectile: entry
                    .field_collision
                    .is_some_and(|(_, projectile)| projectile),
                pickupable: entry.field_collision.is_none(),
                bound: false,
                count: item.quantity.clamp(1, 100),
                sub_type: 0,
            };
            if entry.field_collision.is_some() {
                let Ok(cell) = crate::durability::spell_items_abi::decode_ground_cell(
                    &item.ground.spatial_position,
                ) else {
                    continue;
                };
                let Some(floor) = i16::try_from(cell.z).ok().and_then(i16::checked_neg) else {
                    continue;
                };
                item.ground.spatial_position = ground_cell(cell.x, cell.y, floor);
            }
            let id = item.item_instance_id;
            if overlay.add_ground(item).is_ok() {
                facts.insert(id, entry_facts);
            }
        }
        Self {
            overlay,
            facts,
            field_pin,
        }
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
    fn base_hidden(&self, key: u64) -> bool {
        self.inner.base_hidden(key)
    }
    fn base_text(&self, key: u64) -> Option<&str> {
        self.inner.base_text(key)
    }
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
    pending_origins: std::collections::BTreeMap<u64, CommandRef>,
}

/// The Channel's loop Ground overlay, read once after each change. A leaf lock, never held
/// across an await.
#[derive(Debug, Default)]
pub struct LoopGroundCache {
    state: Mutex<CacheState>,
}

/// Cancellation after a COMMIT must also discard any pre-commit map projection.
pub(super) struct GroundChange<'a>(pub(super) Option<&'a LoopGroundCache>);
impl Drop for GroundChange<'_> {
    fn drop(&mut self) {
        if let Some(cache) = self.0 {
            cache.invalidate();
        }
    }
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
/// A view may overlap the same connection's inventory refresh on the bounded
/// durability holder. Retry only that temporary resource refusal, within one
/// deadline; stored-state and authority errors still close the view.
async fn projection_read<T, E, F, Fut>(mut read: F, busy: impl Fn(&E) -> bool) -> Option<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            match read().await {
                Ok(value) => return Some(value),
                Err(error) if busy(&error) => {
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await
                }
                Err(_) => return None,
            }
        }
    })
    .await
    .ok()
    .flatten()
}

/// The Channel's loop Ground overlay over the bundle `base`; `None` without a pinned index or
/// when the item owner cannot be read.
pub async fn loop_ground(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    base: &Arc<WorldBase>,
    index: Option<&ItemDefinitionIndex>,
) -> Option<Arc<LoopGroundOverlay>> {
    let field_pin = {
        let runtime = admission.runtime.lock().await;
        (
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
            runtime.binding().scope_generation().get(),
        )
    };
    let (epoch, cached) = admission.states.loop_ground.cached();
    if let Some(cached) = cached.filter(|cached| {
        cached.field_pin == field_pin && cached.overlay.base().digest() == base.digest()
    }) {
        return Some(cached);
    }
    let index = index?;
    let items = projection_read(
        || {
            admission
                .root
                .read_item_loop_ground(admission.world_id, admission.channel_id)
        },
        |error| matches!(error, crate::durability::DurabilityError::RootUnavailable),
    )
    .await?;
    let taken = projection_read(
        || {
            admission.root.read_map_item_mint_placements(
                admission.world_id,
                admission.channel_id,
                base.digest(),
                admission.reset_epoch,
                base.entry_count(),
            )
        },
        |error| {
            matches!(
                error,
                crate::durability::map_item_mint::MapItemMintError::Unavailable(
                    crate::durability::DurabilityError::RootUnavailable
                )
            )
        },
    )
    .await?;
    let mut built = LoopGroundOverlay::build(
        base.clone(),
        admission.world_id,
        admission.channel_id,
        index,
        field_pin,
        items,
    );
    crate::map::overlay::pickup::rehide_taken_origins(&mut built.overlay, &taken).ok()?;
    let pending: Vec<_> = admission
        .states
        .loop_ground
        .state
        .lock()
        .ok()?
        .pending_origins
        .keys()
        .copied()
        .collect();
    crate::map::overlay::pickup::rehide_taken_origins(&mut built.overlay, &pending).ok()?;
    let built = Arc::new(built);
    admission.states.loop_ground.store(epoch, &built);
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
/// unsupported source is refused before any durable write; a Ground cell out
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
    if !admission.states.player_trades.physical(
        command.game_session_id,
        match request.item {
            ItemKey::Instance(id) => Some(id),
            _ => None,
        },
    ) {
        return refused(LoopMoveRefusal::Rejected);
    }
    if matches!(request.item, ItemKey::MapBase { .. }) {
        let result = map_pickup::move_map_item(admission, actor, command, request, &permit).await;
        if result == LoopMoveResult::Committed {
            admission
                .states
                .browse_fields
                .committed_move(request.item, None);
        }
        return result;
    }
    let ItemKey::Instance(item) = request.item else {
        return refused(LoopMoveRefusal::NotSupported);
    };
    let (native, pins, index, mailbox) = {
        let mut runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        let dead = states.is_dead(actor);
        let pending = states.has_pending_spell_commit(actor, command.game_session_id);
        drop(states);
        let native = native_for_move(&runtime, admission.active_generation, dead, pending);
        let index = pinned_index(&runtime, admission.active_generation);
        let pin = runtime.content_pin();
        let content_revision = sha256_revision(pin.server_artifact_digest());
        let placement_context = pin.frame_binding_digest().to_vec();
        let mut mailbox = None;
        let pins =
            super::bundle_map_position(&mut runtime, admission.movement_cells, actor).map(|at| {
                // MAIL: a move onto a tile holding a mailbox posts into it.
                if let (LoopTarget::Ground(to), Some(index)) = (&request.target, index)
                    && let (Ok(x), Ok(y), Ok(floor)) = (
                        u16::try_from(to.x),
                        u16::try_from(to.y),
                        i8::try_from(to.floor),
                    )
                {
                    mailbox = super::mail::mailbox_at(at.map, index, TilePos { x, y, floor });
                }
                (
                    at.map.overlay.base().clone(),
                    (at.at.x, at.at.y, at.at.floor),
                    content_revision,
                    placement_context,
                )
            });
        (native, pins, index, mailbox)
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
            if let Some(mailbox) = mailbox {
                if !in_reach(*at, (to.x, to.y, to.floor)) {
                    return LoopMoveResult::TooFar;
                }
                if let Some(posted) =
                    super::mail::post_moved(admission, fence, command.command_id, item, mailbox)
                        .await
                {
                    drop(permit);
                    return posted;
                }
            }
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
            &NativeItemWeights(native).with_wheel(admission.premium_coordinator),
            LoopMoveRequest {
                item,
                count: request.count,
                pickup_cell,
                destination,
            },
        )
        .await;
    // Publish the tile-order event while the same Channel lane still excludes
    // observers. Even pickup + re-drop between two view polls must append the item.
    if touches_ground && matches!(committed, Ok(Ok(_))) {
        let destination = match request.target {
            LoopTarget::Ground(to) => Some(to),
            _ => None,
        };
        admission
            .states
            .browse_fields
            .committed_move(request.item, destination);
    }
    if touches_ground && !matches!(committed, Ok(Err(_))) {
        admission.states.loop_ground.invalidate();
    }
    drop(permit);
    // ATTACK-2: a moved item may have been the swing weapon's container; the next swing reads
    // the equipment again.
    admission
        .states
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

/// A carried container or a container below a proven Ground root, with direct entries.
/// Ground reach uses the actor's current same-floor position under the Channel lane.
/// Reach follows the current Ground root; other external sources remain unavailable.
pub(super) async fn observe_container(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    fence: Option<CurrentCharacterItemFence>,
    target: ItemKey,
) -> Option<ContainerObservation> {
    if matches!(target, ItemKey::BrowseField { .. }) {
        return super::browse_field::observe(admission, actor, fence, target).await;
    }
    let ItemKey::Instance(container) = target else {
        return None;
    };
    let fence = fence?;
    // Retain the Channel lane across current position and durable custody reads.
    // The resulting view is historical data, never permission for a later write.
    let _permit = admission.spell_lane_permit().await?;
    if admission
        .runtime
        .lock()
        .await
        .player_control_facts(actor, fence.game_session_id)
        .is_err()
    {
        return Some(ContainerObservation::Gone);
    }
    let index = pinned_index(
        &*admission.runtime.lock().await,
        admission.active_generation,
    )?;
    let native = admission.active_generation?.native_gameplay()?;
    let carried = admission
        .root
        .read_item_loop_container(admission.character, fence.character_id, container)
        .await
        .ok()?;
    let is_carried = carried.is_some();
    let mut browse_parent = None;
    let (read, reachable) = if let Some(read) = carried {
        (read, true)
    } else {
        // Carried views use the Character owner's custody read, including for
        // sessions admitted without a fresh-admission receipt. Only the new
        // Ground path needs the fresh-session proof for external reach.
        let current = super::FreshAdmissionStore::from_root(admission.root.clone())
            .current_session_at(fence.game_session_id)
            .await
            .ok()?
            .0;
        if current.session_state() != super::GameSessionState::Active
            || super::item_fence_of(current) != Some(fence)
        {
            return Some(ContainerObservation::Gone);
        }
        let (base, at, content_revision, placement_context) = {
            let mut runtime = admission.runtime.lock().await;
            let pin = runtime.content_pin();
            let content_revision = sha256_revision(pin.server_artifact_digest());
            let placement_context = pin.frame_binding_digest().to_vec();
            let at = super::bundle_map_position(&mut runtime, admission.movement_cells, actor)?;
            (
                at.map.overlay.base().clone(),
                (at.at.x, at.at.y, at.at.floor),
                content_revision,
                placement_context,
            )
        };
        let overlay = loop_ground(admission, &base, Some(index)).await?;
        let Some(root) = admission
            .root
            .read_item_loop_ground_container_root(admission.character, fence, container)
            .await
            .ok()?
        else {
            return Some(ContainerObservation::Gone);
        };
        let Some((cell, _)) = overlay.ground_entry(&root) else {
            return Some(ContainerObservation::Gone);
        };
        browse_parent = Some(ItemKey::BrowseField {
            x: i32::from(cell.x),
            y: i32::from(cell.y),
            floor: i16::from(cell.floor),
        });
        let reachable = in_reach(
            at,
            (i32::from(cell.x), i32::from(cell.y), i16::from(cell.floor)),
        );
        let Some(read) = admission
            .root
            .read_item_loop_ground_container(
                admission.character,
                fence,
                container,
                root,
                LoopGround {
                    spatial_position: ground_cell(
                        i32::from(cell.x),
                        i32::from(cell.y),
                        i16::from(cell.floor),
                    ),
                    map_revision: map_revision(&base),
                    content_revision,
                    placement_context,
                },
            )
            .await
            .ok()?
        else {
            return Some(ContainerObservation::Gone);
        };
        (read, reachable)
    };
    // Ground custody reads can wait on physical item/location locks while
    // reconnect advances the durable session. Recheck before publishing.
    if !is_carried {
        let current = super::FreshAdmissionStore::from_root(admission.root.clone())
            .current_session_at(fence.game_session_id)
            .await
            .ok()?
            .0;
        if current.session_state() != super::GameSessionState::Active
            || super::item_fence_of(current) != Some(fence)
        {
            return Some(ContainerObservation::Gone);
        }
    }
    if admission
        .runtime
        .lock()
        .await
        .player_control_facts(actor, fence.game_session_id)
        .is_err()
    {
        return Some(ContainerObservation::Gone);
    }
    let capacity = generation_item_facts(native, &read.container.definition)
        .ok()
        .and_then(|facts| facts.container_capacity)
        .and_then(|capacity| u8::try_from(capacity).ok())
        .filter(|capacity| (1..=MAX_CONTAINER_VIEW_ENTRIES).contains(&usize::from(*capacity)));
    let Some(capacity) = capacity else {
        // USE uses NotAContainer as evidence of carried custody for consumption.
        return Some(if is_carried {
            ContainerObservation::NotAContainer
        } else {
            ContainerObservation::Gone
        });
    };
    if !reachable {
        return Some(ContainerObservation::TooFar);
    }
    let entries = read
        .entries
        .iter()
        .take(usize::from(capacity))
        .map(|entry| {
            Some(ViewItem {
                key: ItemKey::Instance(entry.item.item_instance_id),
                item_definition_ref: definition_ref(index, &entry.item.definition)?,
                count: NonZeroU32::new(entry.item.quantity)?,
                sub_type: read
                    .fluids
                    .get(&entry.item.item_instance_id)
                    .copied()
                    .map_or(0, u32::from),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ContainerObservation::Container {
        parent: read.parent.map(ItemKey::Instance).or(browse_parent),
        capacity,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_retries_only_temporary_holder_refusal() -> Result<(), Box<dyn std::error::Error>>
    {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let attempts = std::cell::Cell::new(0);
                let result = projection_read(
                    || {
                        let attempt = attempts.get() + 1;
                        attempts.set(attempt);
                        std::future::ready(if attempt < 3 { Err(true) } else { Ok(42) })
                    },
                    |temporary| *temporary,
                )
                .await;
                assert_eq!(result, Some(42));
                assert_eq!(attempts.get(), 3);
                let attempts = std::cell::Cell::new(0);
                let result = projection_read(
                    || {
                        attempts.set(attempts.get() + 1);
                        std::future::ready(Err::<u8, _>(false))
                    },
                    |temporary| *temporary,
                )
                .await;
                assert_eq!(result, None);
                assert_eq!(
                    attempts.get(),
                    1,
                    "an authority or stored-state refusal is never retried as resource contention"
                );
            });
        Ok(())
    }

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
