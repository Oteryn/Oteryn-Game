//! Admission against the actual Movement, native map, LocalObject and durable Item owners.
//!
//! A source-qualified spell proposal is not world authority. In particular the current
//! cell index contains collision only, not ground ids, rope spots or floor-change flags.
//! Detached planner snapshots grant no permission. The transaction-borrowing proof below
//! is the sole production producer for the carrier's sealed relocation seam. The accepted
//! entry-r1 source omits tile metadata and therefore still refuses these casts.

use super::native::CompiledNativeSpell;
use super::native_house_movement::HouseMovementPlan;
use crate::content::static_cell_engine::StaticCellEngineError;
use crate::content::{CollisionClass, LogicalCell, NativeEntryMovementCells};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, MovementPositionSnapshot,
};

/// The transaction and current LocalObject owner remain borrowed until the actual
/// position carrier consumes this proof. No public constructor accepts a raw target.
pub(crate) struct SpellRelocationProof<'owner, 'database> {
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    _transaction:
        std::marker::PhantomData<&'owner mut sqlx::Transaction<'database, sqlx::Postgres>>,
    _objects: WorldStaticOwnerBorrow<'owner>,
}

impl crate::foundation::relocation_seal::Sealed for SpellRelocationProof<'_, '_> {}
impl crate::foundation::SpellRelocationProof for SpellRelocationProof<'_, '_> {
    fn parts(
        &self,
    ) -> (
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
    ) {
        (self.actor, self.expected, self.destination)
    }
}

pub(crate) enum WorldRelocationOutcome<'owner, 'database> {
    Move(SpellRelocationProof<'owner, 'database>),
    /// The donor Rope callback ignores a failed teleport and still consumes costs.
    RopeTeleportDenied,
    /// Source House kick also succeeds when its authored entry rejects teleport.
    HouseTeleportDenied,
    Refused {
        reason: &'static str,
    },
}

#[derive(Debug)]
pub(crate) enum WorldRelocationError {
    Admission(Error),
    Owner(crate::world_runtime::WorldRuntimeError),
    Items(crate::durability::spell_item_transaction::SpellItemError),
    MissingTileMetadata,
    MissingSourceGroundIdentity,
    MissingCurrentTopItemIdentities,
    MissingFacing,
    MissingDynamicProjectileFlags,
    GroundAddress(&'static str),
    AuthorityMismatch,
    House(super::house_execution::Error),
}

/// Static map semantics read under the actual runtime/LocalObject owner borrow.
/// Current durable fields must additionally be read by their transaction owner.
pub(crate) struct QualifiedCombatTileFact<'owner> {
    position: MovementLocalPosition,
    tile: &'owner crate::content::QualifiedSpellTile,
    _runtime: &'owner ChannelRuntimeV1,
    _objects: WorldStaticOwnerBorrow<'owner>,
}
impl QualifiedCombatTileFact<'_> {
    /// Read-only equality to the actual privately borrowed physical owner.
    /// This does not mint authority or release the SQL/LocalObject owner locks.
    pub(crate) fn matches_runtime(&self, runtime: &ChannelRuntimeV1) -> bool {
        let expected = self._runtime.binding();
        let actual = runtime.binding();
        let ep = self._runtime.content_pin();
        let ap = runtime.content_pin();
        expected.world_id() == actual.world_id()
            && expected.channel_id() == actual.channel_id()
            && expected.scope_generation() == actual.scope_generation()
            && ep.server_artifact_digest() == ap.server_artifact_digest()
            && ep.map_revision_digest() == ap.map_revision_digest()
            && ep.frame_binding_digest() == ap.frame_binding_digest()
    }
    pub(crate) fn position(&self) -> MovementLocalPosition {
        self.position
    }
    pub(crate) fn qualified_tile(&self) -> &crate::content::QualifiedSpellTile {
        self.tile
    }
    pub(crate) fn no_pvp_zone(&self) -> Option<bool> {
        self.tile.no_pvp_zone()
    }
    pub(crate) fn pvp_zone(&self) -> Option<bool> {
        self.tile.pvp_zone()
    }
    pub(crate) fn ground_present(&self) -> bool {
        self.tile.ground_present()
    }
    pub(crate) fn block_solid(&self) -> bool {
        self.tile.flags().block_solid
    }
    pub(crate) fn block_projectile(&self) -> bool {
        self.tile.flags().block_projectile
    }
    pub(crate) fn floor_change(&self) -> bool {
        self.tile.flags().floor_change
    }
    pub(crate) fn protection_zone(&self) -> bool {
        self.tile.flags().protection_zone
    }
}

pub(crate) fn qualified_combat_tile<'owner>(
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &'owner ChannelRuntimeV1,
    objects: &'owner crate::world_runtime::LocalObjectRuntime,
    position: MovementLocalPosition,
) -> Result<QualifiedCombatTileFact<'owner>, WorldRelocationError> {
    let static_owner = qualified_static_owner(room, runtime, objects)?;
    let tile = tile(room, position)?.ok_or(WorldRelocationError::MissingTileMetadata)?;
    // The existing LocalObject owner exposes movement collision only. That
    // cannot authorize a projectile through an unqualified dynamic obstacle.
    if static_owner.blocks_movement(position) {
        return Err(WorldRelocationError::MissingDynamicProjectileFlags);
    }
    Ok(QualifiedCombatTileFact {
        position,
        tile,
        _runtime: runtime,
        _objects: static_owner,
    })
}

/// The source candidate supplies its genuinely qualified immutable map owner.
/// Source mutable stacks remain Unknown and must be materialized by their own
/// owner; the original four-cell door cannot supply facts about this new map.
enum WorldStaticOwnerBorrow<'owner> {
    Entry(&'owner crate::world_runtime::LocalObjectRuntime),
    Source(&'owner crate::content::native_spell_world::QualifiedNativeSpellWorld),
}
impl WorldStaticOwnerBorrow<'_> {
    fn blocks_movement(&self, position: MovementLocalPosition) -> bool {
        match self {
            Self::Entry(objects) => objects.blocking_cells().contains(&cell(position)),
            Self::Source(world) => matches!(
                world.lookup(world.scope(), cell(position)),
                Ok(CollisionClass::Blocked)
            ),
        }
    }
}
fn qualified_static_owner<'owner>(
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    objects: &'owner crate::world_runtime::LocalObjectRuntime,
) -> Result<WorldStaticOwnerBorrow<'owner>, WorldRelocationError> {
    let pin = runtime.content_pin();
    let cells = room.movement_cells();
    if cells.scope().world_id != runtime.binding().world_id()
        || room.compiled().server_digest() != pin.server_artifact_digest()
        || room.compiled().client_digest() != pin.client_artifact_digest()
        || room.frame_binding().digest() != pin.frame_binding_digest()
        || room.map_revision_digest() != pin.map_revision_digest()
        || cells.scope().generation_digest != pin.server_artifact_digest()
    {
        return Err(WorldRelocationError::AuthorityMismatch);
    }
    if let Some(world) = cells.source_world() {
        if world.scope() != cells.scope() {
            return Err(WorldRelocationError::AuthorityMismatch);
        }
        return Ok(WorldStaticOwnerBorrow::Source(world));
    }
    objects
        .check_scope_fence(
            crate::foundation::RuntimeScopeRefV1::Channel {
                world_id: runtime.binding().world_id(),
                channel_id: runtime.binding().channel_id(),
            },
            runtime.binding().scope_generation(),
        )
        .map_err(WorldRelocationError::Owner)?;
    let door = room.door();
    if door.world_id != cells.scope().world_id
        || door.coordinate_frame != cells.scope().coordinate_frame
        || door.content_lock != cells.scope().content_lock
        || objects.content_generation()
            != &crate::world_runtime::ReferenceContentGeneration::from_content(door)
                .map_err(WorldRelocationError::Owner)?
    {
        return Err(WorldRelocationError::AuthorityMismatch);
    }
    Ok(WorldStaticOwnerBorrow::Entry(objects))
}

fn tile<'a>(
    room: &'a crate::content::QualifiedNativeEntryRoom,
    position: MovementLocalPosition,
) -> Result<Option<&'a crate::content::QualifiedSpellTile>, WorldRelocationError> {
    use crate::content::SpellTileLookupError;
    let cells = room.movement_cells();
    match cells.spell_tiles().lookup(cells.scope(), cell(position)) {
        Ok(tile) => Ok(Some(tile)),
        Err(SpellTileLookupError::Absent) => Ok(None),
        Err(SpellTileLookupError::Unknown) => Err(WorldRelocationError::MissingTileMetadata),
        Err(SpellTileLookupError::ScopeMismatch) => Err(WorldRelocationError::AuthorityMismatch),
    }
}

/// Qualifies the immutable part of a captured mutable cell only after each
/// authored placement has an actual committed map-initialization cause in the
/// same current scope/content. Current Item rows must still be read under this
/// transaction; receipts never assert that a picked-up item is still present.
async fn initialized_tile_in_transaction<'owner>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    position: MovementLocalPosition,
) -> Result<Option<&'owner crate::content::QualifiedSpellTile>, WorldRelocationError> {
    use sqlx::Row;
    match tile(room, position) {
        Ok(tile) => return Ok(tile),
        Err(WorldRelocationError::MissingTileMetadata) => {}
        Err(error) => return Err(error),
    }
    let world = room
        .source_world()
        .ok_or(WorldRelocationError::MissingTileMetadata)?;
    let target = super::world_items_execution::SpellGroundTarget::for_native_tile_read(
        room, runtime, position,
    )
    .map_err(WorldRelocationError::GroundAddress)?;
    let mut count = 0;
    for placement in world
        .pending_mutable_placements()
        .iter()
        .filter(|p| p.cell() == cell(position))
    {
        // Beds, teleports and source doors require their real interaction owner;
        // neither a row nor a map receipt can supply that missing behavior.
        if !matches!(
            placement.source_kind(),
            None | Some("door") | Some("container")
        ) || placement.has_height()
        {
            return Err(WorldRelocationError::MissingTileMetadata);
        }
        let key = format!(
            "{}:cell({},{},{}):ordinal{}",
            crate::content::native_spell_world::SOURCE_WORLD_MAP,
            position.x,
            position.y,
            position.floor,
            placement.ordinal()
        );
        let row=sqlx::query("SELECT r.source_binding,r.spatial_position,r.map_revision,r.content_revision,r.placement_context,COALESCE((SELECT MAX(ad.ownership_generation) FROM game_native_map_scope_adoptions ad WHERE ad.source_transaction_id=r.transaction_id),r.scope_generation)::text FROM game_native_map_item_receipts r JOIN game_native_map_item_audit a USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id WHERE r.world_id=encode($1,'hex')::uuid AND r.channel_id=encode($2,'hex')::uuid AND r.content_digest=$3 AND r.placement_key=$4 AND a.envelope=r.source_intent FOR SHARE OF i")
            .bind(runtime.binding().world_id().as_bytes().as_slice()).bind(runtime.binding().channel_id().as_bytes().as_slice()).bind(runtime.content_pin().server_artifact_digest().as_slice()).bind(key).fetch_optional(&mut **tx).await.map_err(crate::durability::spell_item_transaction::SpellItemError::from).map_err(WorldRelocationError::Items)?
            .ok_or(WorldRelocationError::MissingTileMetadata)?;
        let expected = serde_json::to_vec(placement.binding())
            .map_err(|_| WorldRelocationError::AuthorityMismatch)?;
        let matches = || -> Result<bool, sqlx::Error> {
            Ok(row.try_get::<Vec<u8>, _>(0)? == expected
                && row.try_get::<Vec<u8>, _>(1)? == target.spatial_position
                && row.try_get::<String, _>(2)? == target.map_revision
                && row.try_get::<String, _>(3)? == target.content_revision
                && row.try_get::<Vec<u8>, _>(4)? == target.placement_context
                && row.try_get::<String, _>(5)?
                    == runtime.binding().scope_generation().get().to_string())
        };
        if !matches()
            .map_err(crate::durability::spell_item_transaction::SpellItemError::from)
            .map_err(WorldRelocationError::Items)?
        {
            return Err(WorldRelocationError::AuthorityMismatch);
        }
        count += 1;
    }
    if count == 0 {
        return Err(WorldRelocationError::MissingTileMetadata);
    }
    world
        .immutable_tile(world.scope(), cell(position))
        .map(Some)
        .map_err(|_| WorldRelocationError::MissingTileMetadata)
}

async fn tile_in_transaction<'owner>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &crate::durability::spell_item_transaction::SpellItemAuthority,
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    position: MovementLocalPosition,
) -> Result<Option<&'owner crate::content::QualifiedSpellTile>, WorldRelocationError> {
    crate::durability::spell_item_transaction::check_transaction(tx, authority)
        .await
        .map_err(WorldRelocationError::Items)?;
    if authority.runtime_scope()
        != crate::foundation::RuntimeScopeRefV1::channel(
            runtime.binding().world_id(),
            runtime.binding().channel_id(),
        )
        || authority.scope_generation() != runtime.binding().scope_generation().get()
        || authority.compatible_content_digest() != runtime.content_pin().server_artifact_digest()
    {
        return Err(WorldRelocationError::AuthorityMismatch);
    }
    initialized_tile_in_transaction(tx, room, runtime, position).await
}

/// A STEP consumer must first obtain the actual low-level standing read in this
/// same held transaction. A detached previous tick read cannot call this port.
pub(crate) async fn qualified_standing_tile_in_transaction<'owner>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    read: &crate::durability::spell_item_transaction::StandingTileRead,
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &'owner ChannelRuntimeV1,
    objects: &'owner crate::world_runtime::LocalObjectRuntime,
    position: MovementLocalPosition,
) -> Result<QualifiedCombatTileFact<'owner>, WorldRelocationError> {
    read.check_transaction(tx)
        .await
        .map_err(WorldRelocationError::Items)?;
    if read.fence().runtime_scope
        != crate::foundation::RuntimeScopeRefV1::channel(
            runtime.binding().world_id(),
            runtime.binding().channel_id(),
        )
        || read.fence().scope_ownership_generation != runtime.binding().scope_generation()
        || read.content_digest() != runtime.content_pin().server_artifact_digest()
    {
        return Err(WorldRelocationError::AuthorityMismatch);
    }
    let target = super::world_items_execution::SpellGroundTarget::for_native_tile_read(
        room, runtime, position,
    )
    .map_err(WorldRelocationError::GroundAddress)?;
    if read.target() != &target {
        return Err(WorldRelocationError::AuthorityMismatch);
    }
    let owner = qualified_static_owner(room, runtime, objects)?;
    let tile = initialized_tile_in_transaction(tx, room, runtime, position)
        .await?
        .ok_or(WorldRelocationError::MissingTileMetadata)?;
    if owner.blocks_movement(position) {
        return Err(WorldRelocationError::MissingDynamicProjectileFlags);
    }
    Ok(QualifiedCombatTileFact {
        position,
        tile,
        _runtime: runtime,
        _objects: owner,
    })
}

pub(crate) async fn qualified_combat_tile_in_transaction<'owner>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &crate::durability::spell_item_transaction::SpellItemAuthority,
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &'owner ChannelRuntimeV1,
    objects: &'owner crate::world_runtime::LocalObjectRuntime,
    position: MovementLocalPosition,
) -> Result<QualifiedCombatTileFact<'owner>, WorldRelocationError> {
    let owner = qualified_static_owner(room, runtime, objects)?;
    let tile = tile_in_transaction(tx, authority, room, runtime, position)
        .await?
        .ok_or(WorldRelocationError::MissingTileMetadata)?;
    if owner.blocks_movement(position) {
        return Err(WorldRelocationError::MissingDynamicProjectileFlags);
    }
    Ok(QualifiedCombatTileFact {
        position,
        tile,
        _runtime: runtime,
        _objects: owner,
    })
}

fn offset(
    source: MovementLocalPosition,
    x: i32,
    y: i32,
    floor: i16,
) -> Result<MovementLocalPosition, WorldRelocationError> {
    Ok(MovementLocalPosition {
        x: source
            .x
            .checked_add(x)
            .ok_or(WorldRelocationError::Admission(Error::CoordinateOverflow))?,
        y: source
            .y
            .checked_add(y)
            .ok_or(WorldRelocationError::Admission(Error::CoordinateOverflow))?,
        floor: source
            .floor
            .checked_add(floor)
            .ok_or(WorldRelocationError::Admission(Error::CoordinateOverflow))?,
    })
}

async fn read_items(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &crate::durability::spell_item_transaction::SpellItemAuthority,
    room: &crate::content::QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    position: MovementLocalPosition,
) -> Result<super::world_items_execution::DurableTileItems, WorldRelocationError> {
    let target = super::world_items_execution::SpellGroundTarget::for_native_tile_read(
        room, runtime, position,
    )
    .map_err(WorldRelocationError::GroundAddress)?;
    crate::durability::spell_item_transaction::read_spell_tile_in_transaction(
        tx, authority, &target,
    )
    .await
    .map_err(WorldRelocationError::Items)
}

/// Derives the target from the real current facing and source-qualified tiles, then
/// reads blocking items itself under the sealed current transaction authority. Callers
/// cannot substitute a planner Move, item snapshot, facing or destination. The accepted
/// native entry room is a Channel room; house membership is not supplied by this adapter.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn prepare_relocation<'owner, 'database>(
    spell: &CompiledNativeSpell,
    choice: &str,
    runtime: &mut ChannelRuntimeV1,
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    objects: &'owner crate::world_runtime::LocalObjectRuntime,
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    tx: &'owner mut sqlx::Transaction<'database, sqlx::Postgres>,
    authority: &crate::durability::spell_item_transaction::SpellItemAuthority,
) -> Result<WorldRelocationOutcome<'owner, 'database>, WorldRelocationError> {
    use crate::durability::spell_item_transaction::check_transaction;
    use crate::foundation::{MovementFacing, RuntimeScopeRefV1};
    check_transaction(tx, authority)
        .await
        .map_err(WorldRelocationError::Items)?;
    let current = runtime
        .borrow_movement_position()
        .read(actor)
        .map_err(|e| WorldRelocationError::Admission(Error::Actor(e)))?;
    if current != expected || current.context() != runtime.pinned_movement_context() {
        return Err(WorldRelocationError::Admission(Error::SnapshotMismatch));
    }
    let scope = RuntimeScopeRefV1::Channel {
        world_id: runtime.binding().world_id(),
        channel_id: runtime.binding().channel_id(),
    };
    if authority.runtime_scope() != scope
        || authority.scope_generation() != runtime.binding().scope_generation().get()
        || authority.compatible_content_digest() != runtime.content_pin().server_artifact_digest()
        || runtime
            .positioned_player_for_session(authority.game_session_id())
            .map_err(|e| WorldRelocationError::Admission(Error::Actor(e)))?
            != Some((actor, current))
        || room.movement_cells().scope().world_id != current.world_id()
        || room.compiled().server_digest() != authority.compatible_content_digest()
    {
        return Err(WorldRelocationError::AuthorityMismatch);
    }
    let static_owner = qualified_static_owner(room, runtime, objects)?;
    let cells = room.movement_cells();
    let source = current.position();
    let origin = tile_in_transaction(tx, authority, room, runtime, source)
        .await?
        .ok_or(WorldRelocationError::MissingTileMetadata)?;
    if !origin.ground_present()
        || origin.flags().block_solid
        || (room.source_world().is_none()
            && cells
                .index()
                .lookup(cells.scope(), cell(source))
                .map_err(|e| WorldRelocationError::Admission(Error::SourceCell(e)))?
                != CollisionClass::Walkable)
    {
        return Ok(WorldRelocationOutcome::Refused {
            reason: "not_possible",
        });
    }
    let parameters = &spell.spell()["execution"]["native_behavior"]["parameters"];
    if spell.spell()["execution"]["native_behavior"]["key"].as_str() != Some("vertical_move") {
        return Err(WorldRelocationError::Admission(Error::UnsupportedRecipe));
    }
    let (destination, rope) = match parameters["mode"].as_str() {
        Some("levitate") => {
            let (x, y) = match current
                .facing()
                .ok_or(WorldRelocationError::MissingFacing)?
            {
                MovementFacing::North => (0, -1),
                MovementFacing::East => (1, 0),
                MovementFacing::South => (0, 1),
                MovementFacing::West => (-1, 0),
            };
            let up = match choice.to_ascii_lowercase().as_str() {
                "up" => true,
                "down" => false,
                _ => {
                    return Ok(WorldRelocationOutcome::Refused {
                        reason: "not_possible",
                    });
                }
            };
            if up && source.floor == -8 || !up && source.floor == -7 {
                return Ok(WorldRelocationOutcome::Refused {
                    reason: "not_possible",
                });
            }
            let probe = if up {
                offset(source, 0, 0, 1)?
            } else {
                offset(source, x, y, 0)?
            };
            if let Some(probe_tile) =
                tile_in_transaction(tx, authority, room, runtime, probe).await?
            {
                if probe_tile.ground_present()
                    || if up {
                        probe_tile.flags().immovable_block_solid
                    } else {
                        probe_tile.flags().block_solid
                    }
                {
                    return Ok(WorldRelocationOutcome::Refused {
                        reason: "not_possible",
                    });
                }
            }
            // Even verified source-air cells must read the actual durable item
            // owner under the same transaction. Static absence never proves an
            // empty current item stack or permits ignoring legacy owner rows.
            let items = read_items(tx, authority, room, runtime, probe).await?;
            if items.items.iter().any(|item| {
                if up {
                    item.immovable_block_solid
                } else {
                    item.blocks_movement
                }
            }) {
                return Ok(WorldRelocationOutcome::Refused {
                    reason: "not_possible",
                });
            }
            (offset(source, x, y, if up { 1 } else { -1 })?, false)
        }
        Some("rope_up") => {
            if !origin.ground_present() {
                return Ok(WorldRelocationOutcome::Refused {
                    reason: "not_possible",
                });
            }
            let ground_match = origin.ground_source_id().is_some_and(|ground| {
                parameters["rope_ground_items"]
                    .as_array()
                    .is_some_and(|ids| ids.iter().any(|id| id.as_u64() == Some(u64::from(ground))))
            });
            let top_match = parameters["rope_top_items"].as_array().is_some_and(|ids| {
                origin
                    .top_source_ids()
                    .iter()
                    .any(|top| ids.iter().any(|id| id.as_u64() == Some(u64::from(*top))))
            });
            if !ground_match && !top_match {
                if origin.ground_source_id().is_none() {
                    return Err(WorldRelocationError::MissingSourceGroundIdentity);
                }
                if !read_items(tx, authority, room, runtime, source)
                    .await?
                    .items
                    .is_empty()
                {
                    return Err(WorldRelocationError::MissingCurrentTopItemIdentities);
                }
                return Ok(WorldRelocationOutcome::Refused {
                    reason: "not_possible",
                });
            }
            let mut selected = None;
            for (x, y) in [
                (0, 1),
                (0, -1),
                (1, 0),
                (-1, 0),
                (-1, 0),
                (-1, 1),
                (1, 1),
                (-1, -1),
                (1, -1),
            ] {
                let candidate = offset(source, x, y, 1)?;
                let Some(t) = tile_in_transaction(tx, authority, room, runtime, candidate).await?
                else {
                    continue;
                };
                let f = t.flags();
                if !t.ground_present()
                    || f.block_solid
                    || f.block_projectile
                    || f.immovable_block_solid
                    || f.immovable_block_item
                    || f.immovable_nonfield_block_item
                {
                    continue;
                }
                let items = read_items(tx, authority, room, runtime, candidate).await?;
                if static_owner.blocks_movement(candidate)
                    || items.items.iter().any(|item| {
                        item.blocks_movement || item.blocks_projectile || item.immovable_block_solid
                    })
                {
                    continue;
                }
                selected = Some(candidate);
                break;
            }
            let destination = if let Some(selected) = selected {
                selected
            } else {
                let fallback = offset(source, 0, 1, 1)?;
                if tile_in_transaction(tx, authority, room, runtime, fallback)
                    .await?
                    .is_none()
                {
                    return Ok(WorldRelocationOutcome::Refused {
                        reason: "not_enough_room",
                    });
                }
                fallback
            };
            (destination, true)
        }
        _ => return Err(WorldRelocationError::Admission(Error::UnsupportedRecipe)),
    };
    let Some(target) = tile_in_transaction(tx, authority, room, runtime, destination).await? else {
        return Ok(WorldRelocationOutcome::Refused {
            reason: "not_possible",
        });
    };
    let collision = if room.source_world().is_some() {
        if target.ground_present() && !target.flags().block_solid {
            CollisionClass::Walkable
        } else {
            CollisionClass::Blocked
        }
    } else {
        cells
            .index()
            .lookup(cells.scope(), cell(destination))
            .map_err(|e| WorldRelocationError::Admission(Error::SourceCell(e)))?
    };
    let items = read_items(tx, authority, room, runtime, destination).await?;
    let denied = collision != CollisionClass::Walkable
        || static_owner.blocks_movement(destination)
        || if rope {
            !target.ground_present()
                || target.flags().block_solid
                || target.flags().immovable_block_solid
                || target.flags().immovable_block_item
                || target.flags().immovable_nonfield_block_item
                || items
                    .items
                    .iter()
                    .any(|item| item.blocks_movement || item.immovable_block_solid)
                || runtime
                    .position_occupied_by_other(actor, destination)
                    .map_err(|e| WorldRelocationError::Admission(Error::Actor(e)))?
        } else {
            !target.ground_present()
                || target.flags().immovable_block_solid
                || target.flags().floor_change
                || items.items.iter().any(|item| item.immovable_block_solid)
        };
    if denied {
        return Ok(if rope {
            WorldRelocationOutcome::RopeTeleportDenied
        } else {
            WorldRelocationOutcome::Refused {
                reason: "not_possible",
            }
        });
    }
    check_transaction(tx, authority)
        .await
        .map_err(WorldRelocationError::Items)?;
    Ok(WorldRelocationOutcome::Move(SpellRelocationProof {
        actor,
        expected: current,
        destination,
        _transaction: std::marker::PhantomData,
        _objects: static_owner,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerticalRecipe {
    Levitate,
    MagicRope,
}

/// Resolves Aleta itself under the actual map/admission/ACL owners, then seals
/// only that target House entry. No caller Move or raw destination is accepted.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn prepare_house_kick<'owner, 'database>(
    spell: &CompiledNativeSpell,
    name: &str,
    runtime: &mut ChannelRuntimeV1,
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    objects: &'owner crate::world_runtime::LocalObjectRuntime,
    admissions: &crate::durability::fresh_admission::FreshAdmissionStore,
    caster: ExactActorRef,
    expected_caster: MovementPositionSnapshot,
    tx: &'owner mut sqlx::Transaction<'database, sqlx::Postgres>,
    authority: &crate::durability::spell_item_transaction::SpellItemAuthority,
) -> Result<WorldRelocationOutcome<'owner, 'database>, WorldRelocationError> {
    use crate::durability::spell_item_transaction::check_transaction;
    let authorization = super::house_execution::authorize_aleta_kick_in_transaction(
        tx,
        authority,
        admissions,
        room,
        runtime,
        spell,
        caster,
        expected_caster,
        name,
    )
    .await
    .map_err(WorldRelocationError::House)?;
    let (actor, expected, destination) = authorization.parts();
    if runtime
        .read_actor_position(actor)
        .map_err(|e| WorldRelocationError::Admission(Error::Actor(e)))?
        != expected
    {
        return Err(WorldRelocationError::AuthorityMismatch);
    }
    let static_owner = qualified_static_owner(room, runtime, objects)?;
    let cells = room.movement_cells();
    let collision = cells
        .index()
        .lookup(cells.scope(), cell(destination))
        .map_err(|e| WorldRelocationError::Admission(Error::SourceCell(e)))?;
    let target = tile_in_transaction(tx, authority, room, runtime, destination)
        .await?
        .ok_or(WorldRelocationError::MissingTileMetadata)?;
    let items = read_items(tx, authority, room, runtime, destination).await?;
    if collision != CollisionClass::Walkable
        || !target.ground_present()
        || target.flags().block_solid
        || target.flags().immovable_block_solid
        || static_owner.blocks_movement(destination)
        || items
            .items
            .iter()
            .any(|item| item.blocks_movement || item.immovable_block_solid)
        || runtime
            .position_occupied_by_other(actor, destination)
            .map_err(|e| WorldRelocationError::Admission(Error::Actor(e)))?
    {
        return Ok(WorldRelocationOutcome::HouseTeleportDenied);
    }
    check_transaction(tx, authority)
        .await
        .map_err(WorldRelocationError::Items)?;
    Ok(WorldRelocationOutcome::Move(SpellRelocationProof {
        actor,
        expected,
        destination,
        _transaction: std::marker::PhantomData,
        _objects: static_owner,
    }))
}

/// Data the loaded-world owner must provide under the same scope/generation and
/// owner turn as the eventual entry check. None is available from CollisionClass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MissingTileFact {
    GroundItemIdentity,
    TopItemIdentities,
    GroundPresence,
    BlockSolid,
    ImmovableBlockSolid,
    BlockProjectile,
    ImmovableBlockItem,
    ImmovableNonFieldBlockItem,
    FloorChange,
    CurrentRoomKind,
    CurrentCasterFacing,
    CurrentEntryPermissions,
}

const LEVITATE_FACTS: &[MissingTileFact] = &[
    MissingTileFact::GroundPresence,
    MissingTileFact::BlockSolid,
    MissingTileFact::ImmovableBlockSolid,
    MissingTileFact::FloorChange,
    MissingTileFact::CurrentCasterFacing,
    MissingTileFact::CurrentRoomKind,
    MissingTileFact::CurrentEntryPermissions,
];
const ROPE_FACTS: &[MissingTileFact] = &[
    MissingTileFact::ImmovableBlockSolid,
    MissingTileFact::GroundItemIdentity,
    MissingTileFact::TopItemIdentities,
    MissingTileFact::GroundPresence,
    MissingTileFact::BlockSolid,
    MissingTileFact::BlockProjectile,
    MissingTileFact::ImmovableBlockItem,
    MissingTileFact::ImmovableNonFieldBlockItem,
    MissingTileFact::CurrentRoomKind,
    MissingTileFact::CurrentEntryPermissions,
];

/// A blocked admission report, never a movement capability or successful cast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissingWorldTileEligibility {
    pub(crate) recipe: VerticalRecipe,
    pub(crate) source: MovementLocalPosition,
    pub(crate) destination: MovementLocalPosition,
    /// Actual map evidence is retained even when the destination is outside the
    /// current qualified room. A missing cell is not treated as walkable.
    pub(crate) destination_collision: Result<CollisionClass, StaticCellEngineError>,
    pub(crate) missing: &'static [MissingTileFact],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    Actor(CarrierError),
    SnapshotMismatch,
    ContextMismatch,
    WorldMismatch,
    ContentGenerationMismatch,
    UnsupportedRecipe,
    InvalidMoveProposal,
    InvalidGeometry,
    CoordinateOverflow,
    SourceCell(StaticCellEngineError),
    SourceBlocked,
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for Error {}

fn cell(position: MovementLocalPosition) -> LogicalCell {
    LogicalCell {
        x: position.x,
        y: position.y,
        z: i32::from(position.floor),
    }
}

/// Source z increases downward; the native floor increases upward (ADR-0021).
fn native_destination(
    position: super::native_house_movement::Position,
) -> Result<MovementLocalPosition, Error> {
    let floor = position.z.checked_neg().ok_or(Error::CoordinateOverflow)?;
    Ok(MovementLocalPosition {
        x: position.x,
        y: position.y,
        floor: i16::try_from(floor).map_err(|_| Error::CoordinateOverflow)?,
    })
}

/// Reads the real actor slot through its current owner and the runtime's independently
/// pinned content. Both recipes remain blocked by missing world eligibility facts.
/// The supplied proposal is constrained by the canonical recipe and source geometry;
/// even a geometrically valid proposal grants no relocation authority.
pub(crate) fn preflight_relocation(
    spell: &CompiledNativeSpell,
    runtime: &mut ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    proposal: &HouseMovementPlan,
) -> Result<MissingWorldTileEligibility, Error> {
    let current = runtime
        .borrow_movement_position()
        .read(actor)
        .map_err(Error::Actor)?;
    if current != expected {
        return Err(Error::SnapshotMismatch);
    }
    if current.context() != runtime.pinned_movement_context() {
        return Err(Error::ContextMismatch);
    }
    let scope = cells.scope();
    if current.world_id() != runtime.binding().world_id()
        || scope.world_id != runtime.binding().world_id()
    {
        return Err(Error::WorldMismatch);
    }
    if scope.generation_digest != runtime.content_pin().server_artifact_digest() {
        return Err(Error::ContentGenerationMismatch);
    }
    let source = current.position();
    match cells
        .index()
        .lookup(scope, cell(source))
        .map_err(Error::SourceCell)?
    {
        CollisionClass::Walkable => (),
        CollisionClass::Blocked => return Err(Error::SourceBlocked),
    }
    let native = &spell.spell()["execution"]["native_behavior"];
    let recipe = match (
        native["key"].as_str(),
        native["parameters"]["mode"].as_str(),
    ) {
        (Some("vertical_move"), Some("levitate")) => VerticalRecipe::Levitate,
        (Some("vertical_move"), Some("rope_up")) => VerticalRecipe::MagicRope,
        _ => return Err(Error::UnsupportedRecipe),
    };
    let HouseMovementPlan::Move {
        target_is_caster: true,
        destination,
        before_effect,
        after_effect,
        failed_move_counts_as_success,
    } = proposal
    else {
        return Err(Error::InvalidMoveProposal);
    };
    let rope = recipe == VerticalRecipe::MagicRope;
    if *before_effect != if rope { "poff" } else { "none" }
        || *after_effect != "teleport"
        || *failed_move_counts_as_success != rope
    {
        return Err(Error::InvalidMoveProposal);
    }
    let destination = native_destination(*destination)?;
    let dx = i64::from(destination.x) - i64::from(source.x);
    let dy = i64::from(destination.y) - i64::from(source.y);
    let df = i32::from(destination.floor) - i32::from(source.floor);
    let valid = if rope {
        df == 1 && dx.abs() <= 1 && dy.abs() <= 1 && (dx != 0 || dy != 0)
    } else {
        dx.abs() + dy.abs() == 1
            && df.abs() == 1
            && !matches!((source.floor, destination.floor), (-7, -8) | (-8, -7))
    };
    if !valid {
        return Err(Error::InvalidGeometry);
    }
    Ok(MissingWorldTileEligibility {
        recipe,
        source,
        destination,
        destination_collision: cells.index().lookup(scope, cell(destination)),
        missing: if rope { ROPE_FACTS } else { LEVITATE_FACTS },
    })
}

#[cfg(test)]
mod tests {
    // Panicking assertions are confined to regression tests.
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use crate::content::{QualifiedNativeEntryRoom, qualify_native_entry_room};
    use crate::foundation::{ChannelContentPin, ChannelId, GameSessionId, NodeId, WorldId};
    use serde_json::{Value, json};

    fn uuid(raw: u8) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = raw;
        bytes
    }
    fn room(raw: u8) -> QualifiedNativeEntryRoom {
        qualify_native_entry_room(WorldId::decode(&uuid(raw)).unwrap()).unwrap()
    }
    fn runtime(room: &QualifiedNativeEntryRoom, wrong_generation: bool) -> ChannelRuntimeV1 {
        let start = room.entry_start();
        let pin = ChannelContentPin::from_activation(
            room.movement_cells().scope().world_id,
            1,
            if wrong_generation {
                [9; 32]
            } else {
                room.compiled().server_digest()
            },
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        ChannelRuntimeV1::from_committed_assignment(
            room.movement_cells().scope().world_id,
            ChannelId::decode(&uuid(2)).unwrap(),
            NodeId::decode(&uuid(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .unwrap()
    }
    fn admit(runtime: &mut ChannelRuntimeV1, raw: u8) -> (ExactActorRef, MovementPositionSnapshot) {
        let reservation = runtime
            .reserve_fresh_session(GameSessionId::decode(&uuid(raw)).unwrap())
            .unwrap();
        let actor = runtime.commit_fresh_session(reservation).unwrap();
        runtime.initialize_first_entry_position(actor).unwrap();
        let position = runtime.borrow_movement_position().read(actor).unwrap();
        (actor, position)
    }
    fn spell(name: &str) -> CompiledNativeSpell {
        let document: Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
        ))
        .unwrap();
        let profile = document["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|profile| profile["name"] == name)
            .unwrap();
        super::super::native::spell_from_bundle(
            &json!({"spell":profile["spell"]}),
            &profile["dependencies"],
        )
        .unwrap()
    }
    fn proposal(rope: bool) -> HouseMovementPlan {
        HouseMovementPlan::Move {
            target_is_caster: true,
            destination: super::super::native_house_movement::Position { x: 1, y: 0, z: -1 },
            before_effect: if rope { "poff" } else { "none" },
            after_effect: "teleport",
            failed_move_counts_as_success: rope,
        }
    }
    fn assert_unchanged(
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        expected: MovementPositionSnapshot,
    ) {
        assert_eq!(
            runtime.borrow_movement_position().read(actor).unwrap(),
            expected
        );
    }

    #[test]
    fn real_owner_and_real_map_refuse_both_recipes_without_tile_metadata() {
        let room = room(1);
        let mut runtime = runtime(&room, false);
        let (actor, expected) = admit(&mut runtime, 10);
        for (name, rope) in [("Levitate", false), ("Magic Rope", true)] {
            let blocked = preflight_relocation(
                &spell(name),
                &mut runtime,
                room.movement_cells(),
                actor,
                expected,
                &proposal(rope),
            )
            .unwrap();
            assert_eq!(blocked.destination.floor, 1); // legacy -1 becomes native +1
            assert_eq!(
                blocked.destination_collision,
                Err(StaticCellEngineError::Absent)
            );
            assert!(blocked.missing.contains(&MissingTileFact::GroundPresence));
            assert!(
                blocked
                    .missing
                    .contains(&MissingTileFact::CurrentEntryPermissions)
            );
            assert_unchanged(&mut runtime, actor, expected);
        }
    }

    #[test]
    fn foreign_world_and_stale_content_are_refused_without_moving() {
        let room = room(1);
        let foreign = self::room(4);
        let mut runtime = runtime(&room, false);
        let (actor, expected) = admit(&mut runtime, 10);
        assert_eq!(
            preflight_relocation(
                &spell("Levitate"),
                &mut runtime,
                foreign.movement_cells(),
                actor,
                expected,
                &proposal(false)
            ),
            Err(Error::WorldMismatch)
        );
        assert_unchanged(&mut runtime, actor, expected);
        let mut stale = self::runtime(&room, true);
        let (actor, expected) = admit(&mut stale, 11);
        assert_eq!(
            preflight_relocation(
                &spell("Levitate"),
                &mut stale,
                room.movement_cells(),
                actor,
                expected,
                &proposal(false)
            ),
            Err(Error::ContentGenerationMismatch)
        );
        assert_unchanged(&mut stale, actor, expected);
    }

    #[test]
    fn stale_actor_and_foreign_snapshot_are_refused() {
        let room = room(1);
        let mut runtime = runtime(&room, false);
        let (actor, expected) = admit(&mut runtime, 10);
        let (other, other_position) = admit(&mut runtime, 11);
        assert_eq!(
            preflight_relocation(
                &spell("Levitate"),
                &mut runtime,
                room.movement_cells(),
                actor,
                other_position,
                &proposal(false)
            ),
            Err(Error::SnapshotMismatch)
        );
        assert_unchanged(&mut runtime, actor, expected);
        runtime.remove_test_actor(actor).unwrap();
        assert!(matches!(
            preflight_relocation(
                &spell("Levitate"),
                &mut runtime,
                room.movement_cells(),
                actor,
                expected,
                &proposal(false)
            ),
            Err(Error::Actor(_))
        ));
        assert_unchanged(&mut runtime, other, other_position);
    }

    #[test]
    fn actual_map_blocked_source_and_forged_target_fail_without_writes() {
        let room = room(1);
        let mut runtime = runtime(&room, false);
        let (actor, expected) = admit(&mut runtime, 10);
        let mut forged = proposal(false);
        if let HouseMovementPlan::Move { destination, .. } = &mut forged {
            destination.x = 200;
        }
        assert_eq!(
            preflight_relocation(
                &spell("Levitate"),
                &mut runtime,
                room.movement_cells(),
                actor,
                expected,
                &forged
            ),
            Err(Error::InvalidGeometry)
        );
        assert_unchanged(&mut runtime, actor, expected);
        // Negative source fixture through the existing carrier, not a replacement
        // position store. This owner-only write does not claim map entry admission.
        let blocked = runtime
            .borrow_movement_position()
            .commit_cardinal(
                expected,
                MovementLocalPosition {
                    x: 0,
                    y: -1,
                    floor: 0,
                },
            )
            .unwrap();
        assert_eq!(
            preflight_relocation(
                &spell("Levitate"),
                &mut runtime,
                room.movement_cells(),
                actor,
                blocked,
                &proposal(false)
            ),
            Err(Error::SourceBlocked)
        );
        assert_unchanged(&mut runtime, actor, blocked);
    }

    #[test]
    fn actual_native_metadata_unknown_does_not_become_empty_or_clear() {
        let room = room(1);
        let mut runtime = runtime(&room, false);
        let (actor, expected) = admit(&mut runtime, 10);
        assert!(matches!(
            tile(&room, expected.position()),
            Err(WorldRelocationError::MissingTileMetadata)
        ));
        let above = offset(expected.position(), 0, 0, 1).unwrap();
        assert_eq!(
            room.movement_cells()
                .index()
                .lookup(room.movement_cells().scope(), cell(above)),
            Err(StaticCellEngineError::Absent)
        );
        // The whole metadata document is unknown even where no collision cell
        // exists. The spell cannot derive an empty stack or absent ground from it.
        assert!(matches!(
            tile(&room, above),
            Err(WorldRelocationError::MissingTileMetadata)
        ));
        assert_unchanged(&mut runtime, actor, expected);
    }

    #[test]
    fn coordinate_conversion_is_checked_and_native_floor_is_upward() {
        use super::super::native_house_movement::Position;
        assert_eq!(
            native_destination(Position { x: 1, y: 2, z: 7 })
                .unwrap()
                .floor,
            -7
        );
        assert_eq!(
            native_destination(Position {
                x: 1,
                y: 2,
                z: i32::MIN
            }),
            Err(Error::CoordinateOverflow)
        );
        assert_eq!(
            native_destination(Position {
                x: 1,
                y: 2,
                z: 32769
            }),
            Err(Error::CoordinateOverflow)
        );
    }
}
