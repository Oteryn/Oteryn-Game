//! Transaction-bound native tile reads shared by Movement, Creature and Spell owners.
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};

/// Borrow the actual WorldObject owner; implementations preserve its scope/content checks.
pub trait NativeWorldObjectRead {
    type Error;
    fn qualify_entry(
        &self,
        room: &crate::content::QualifiedNativeEntryRoom,
        runtime: &ChannelRuntimeV1,
    ) -> Result<bool, Self::Error>;
    fn blocks_movement(&self, position: MovementLocalPosition) -> bool;
}

#[derive(Debug)]
pub enum NativeWorldReadError<E> {
    Owner(E),
    Items(crate::durability::spell_item_transaction::SpellItemError),
    MissingTileMetadata,
    MissingDynamicProjectileFlags,
    GroundAddress(&'static str),
    AuthorityMismatch,
    CoordinateOverflow,
}

/// Static map semantics read under the actual runtime/LocalObject owner borrow.
/// Current durable fields must additionally be read by their transaction owner.
pub struct QualifiedCombatTileFact<'owner, O: NativeWorldObjectRead + ?Sized> {
    position: MovementLocalPosition,
    tile: &'owner crate::content::QualifiedSpellTile,
    _runtime: &'owner ChannelRuntimeV1,
    _objects: WorldStaticOwnerBorrow<'owner, O>,
}
impl<O: NativeWorldObjectRead + ?Sized> QualifiedCombatTileFact<'_, O> {
    /// Read-only equality to the actual privately borrowed physical owner.
    /// This does not mint authority or release the SQL/LocalObject owner locks.
    pub fn matches_runtime(&self, runtime: &ChannelRuntimeV1) -> bool {
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
    pub fn position(&self) -> MovementLocalPosition {
        self.position
    }
    pub fn qualified_tile(&self) -> &crate::content::QualifiedSpellTile {
        self.tile
    }
    pub fn no_pvp_zone(&self) -> Option<bool> {
        self.tile.no_pvp_zone()
    }
    pub fn pvp_zone(&self) -> Option<bool> {
        self.tile.pvp_zone()
    }
    pub fn ground_present(&self) -> bool {
        self.tile.ground_present()
    }
    pub fn block_solid(&self) -> bool {
        self.tile.flags().block_solid
    }
    pub fn block_projectile(&self) -> bool {
        self.tile.flags().block_projectile
    }
    pub fn floor_change(&self) -> bool {
        self.tile.flags().floor_change
    }
    pub fn protection_zone(&self) -> bool {
        self.tile.flags().protection_zone
    }
}

pub fn qualified_combat_tile<'owner, O: NativeWorldObjectRead + ?Sized>(
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &'owner ChannelRuntimeV1,
    objects: &'owner O,
    position: MovementLocalPosition,
) -> Result<QualifiedCombatTileFact<'owner, O>, NativeWorldReadError<O::Error>> {
    let static_owner = qualified_static_owner(room, runtime, objects)?;
    let tile =
        tile::<O::Error>(room, position)?.ok_or(NativeWorldReadError::MissingTileMetadata)?;
    // The existing LocalObject owner exposes movement collision only. That
    // cannot authorize a projectile through an unqualified dynamic obstacle.
    if static_owner.blocks_movement(position) {
        return Err(NativeWorldReadError::MissingDynamicProjectileFlags);
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
pub enum WorldStaticOwnerBorrow<'owner, O: NativeWorldObjectRead + ?Sized> {
    Entry(&'owner O),
    Source(&'owner crate::content::native_spell_world::QualifiedNativeSpellWorld),
    /// BUNDLE-COMBAT-ENABLE-1: a bundle World's cells. Its static blockers are already the
    /// bundle tile facts (`block_solid`, `block_projectile`, BUNDLE-TILES-1), and its runtime
    /// blockers (a usable door, a pickupable Item) are Unknown tiles, refused before this owner
    /// is asked; the entry door runtime has no cell in the bundle frame.
    Bundle,
}
impl<O: NativeWorldObjectRead + ?Sized> WorldStaticOwnerBorrow<'_, O> {
    pub fn blocks_movement(&self, position: MovementLocalPosition) -> bool {
        match self {
            Self::Bundle => false,
            Self::Entry(objects) => objects.blocks_movement(position),
            Self::Source(world) => matches!(
                world.lookup(world.scope(), cell(position)),
                Ok(CollisionClass::Blocked)
            ),
        }
    }
}
pub fn qualified_static_owner<'owner, O: NativeWorldObjectRead + ?Sized>(
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    objects: &'owner O,
) -> Result<WorldStaticOwnerBorrow<'owner, O>, NativeWorldReadError<O::Error>> {
    let pin = runtime.content_pin();
    let cells = room.movement_cells();
    if cells.scope().world_id != runtime.binding().world_id()
        || room.compiled().server_digest() != pin.server_artifact_digest()
        || room.compiled().client_digest() != pin.client_artifact_digest()
        || room.frame_binding().digest() != pin.frame_binding_digest()
        || room.map_revision_digest() != pin.map_revision_digest()
        || cells.scope().generation_digest != pin.server_artifact_digest()
    {
        return Err(NativeWorldReadError::AuthorityMismatch);
    }
    if let Some(world) = cells.source_world() {
        if world.scope() != cells.scope() {
            return Err(NativeWorldReadError::AuthorityMismatch);
        }
        return Ok(WorldStaticOwnerBorrow::Source(world));
    }
    if matches!(
        cells.index(),
        crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
    ) {
        return Ok(WorldStaticOwnerBorrow::Bundle);
    }
    if !objects
        .qualify_entry(room, runtime)
        .map_err(NativeWorldReadError::Owner)?
    {
        return Err(NativeWorldReadError::AuthorityMismatch);
    }
    Ok(WorldStaticOwnerBorrow::Entry(objects))
}

pub fn tile<E>(
    room: &crate::content::QualifiedNativeEntryRoom,
    position: MovementLocalPosition,
) -> Result<Option<&crate::content::QualifiedSpellTile>, NativeWorldReadError<E>> {
    use crate::content::SpellTileLookupError;
    let cells = room.movement_cells();
    match cells.spell_tiles().lookup(cells.scope(), cell(position)) {
        Ok(tile) => Ok(Some(tile)),
        Err(SpellTileLookupError::Absent) => Ok(None),
        Err(SpellTileLookupError::Unknown) => Err(NativeWorldReadError::MissingTileMetadata),
        Err(SpellTileLookupError::ScopeMismatch) => Err(NativeWorldReadError::AuthorityMismatch),
    }
}

/// Qualifies the immutable part of a captured mutable cell only after each
/// authored placement has an actual committed map-initialization cause in the
/// same current scope/content. Current Item rows must still be read under this
/// transaction; receipts never assert that a picked-up item is still present.
async fn initialized_tile_in_transaction<'owner, E>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    position: MovementLocalPosition,
) -> Result<Option<&'owner crate::content::QualifiedSpellTile>, NativeWorldReadError<E>> {
    use sqlx::Row;
    match tile(room, position) {
        Ok(tile) => return Ok(tile),
        Err(NativeWorldReadError::MissingTileMetadata) => {}
        Err(error) => return Err(error),
    }
    let world = room
        .source_world()
        .ok_or(NativeWorldReadError::MissingTileMetadata)?;
    let target = crate::durability::native_world_items::SpellGroundTarget::for_native_tile_read(
        room, runtime, position,
    )
    .map_err(NativeWorldReadError::GroundAddress)?;
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
            return Err(NativeWorldReadError::MissingTileMetadata);
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
            .bind(runtime.binding().world_id().as_bytes().as_slice()).bind(runtime.binding().channel_id().as_bytes().as_slice()).bind(runtime.content_pin().server_artifact_digest().as_slice()).bind(key).fetch_optional(&mut **tx).await.map_err(crate::durability::spell_item_transaction::SpellItemError::from).map_err(NativeWorldReadError::Items)?
            .ok_or(NativeWorldReadError::MissingTileMetadata)?;
        let expected = serde_json::to_vec(placement.binding())
            .map_err(|_| NativeWorldReadError::AuthorityMismatch)?;
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
            .map_err(NativeWorldReadError::Items)?
        {
            return Err(NativeWorldReadError::AuthorityMismatch);
        }
        count += 1;
    }
    if count == 0 {
        return Err(NativeWorldReadError::MissingTileMetadata);
    }
    world
        .immutable_tile(world.scope(), cell(position))
        .map(Some)
        .map_err(|_| NativeWorldReadError::MissingTileMetadata)
}

pub async fn tile_in_transaction<'owner, E>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &crate::durability::spell_item_transaction::SpellItemAuthority,
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    position: MovementLocalPosition,
) -> Result<Option<&'owner crate::content::QualifiedSpellTile>, NativeWorldReadError<E>> {
    crate::durability::spell_item_transaction::check_transaction(tx, authority)
        .await
        .map_err(NativeWorldReadError::Items)?;
    if authority.runtime_scope()
        != crate::foundation::RuntimeScopeRefV1::channel(
            runtime.binding().world_id(),
            runtime.binding().channel_id(),
        )
        || authority.scope_generation() != runtime.binding().scope_generation().get()
        || authority.compatible_content_digest() != runtime.content_pin().server_artifact_digest()
    {
        return Err(NativeWorldReadError::AuthorityMismatch);
    }
    initialized_tile_in_transaction(tx, room, runtime, position).await
}

/// A STEP consumer must first obtain the actual low-level standing read in this
/// same held transaction. A detached previous tick read cannot call this port.
pub async fn qualified_standing_tile_in_transaction<'owner, O: NativeWorldObjectRead + ?Sized>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    read: &crate::durability::spell_item_transaction::StandingTileRead,
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &'owner ChannelRuntimeV1,
    objects: &'owner O,
    position: MovementLocalPosition,
) -> Result<QualifiedCombatTileFact<'owner, O>, NativeWorldReadError<O::Error>> {
    read.check_transaction(tx)
        .await
        .map_err(NativeWorldReadError::Items)?;
    if read.fence().runtime_scope
        != crate::foundation::RuntimeScopeRefV1::channel(
            runtime.binding().world_id(),
            runtime.binding().channel_id(),
        )
        || read.fence().scope_ownership_generation != runtime.binding().scope_generation()
        || read.content_digest() != runtime.content_pin().server_artifact_digest()
    {
        return Err(NativeWorldReadError::AuthorityMismatch);
    }
    let target = crate::durability::native_world_items::SpellGroundTarget::for_native_tile_read(
        room, runtime, position,
    )
    .map_err(NativeWorldReadError::GroundAddress)?;
    if read.target() != &target {
        return Err(NativeWorldReadError::AuthorityMismatch);
    }
    let owner = qualified_static_owner(room, runtime, objects)?;
    let tile = initialized_tile_in_transaction::<O::Error>(tx, room, runtime, position)
        .await?
        .ok_or(NativeWorldReadError::MissingTileMetadata)?;
    if owner.blocks_movement(position) {
        return Err(NativeWorldReadError::MissingDynamicProjectileFlags);
    }
    Ok(QualifiedCombatTileFact {
        position,
        tile,
        _runtime: runtime,
        _objects: owner,
    })
}

/// Read-only source STEP probes may inspect a known solid neighbour while
/// resolving stair destinations. This fact grants neither combat nor movement.
pub struct QualifiedSourceStepProbe<'owner, O: NativeWorldObjectRead + ?Sized> {
    tile: &'owner crate::content::QualifiedSpellTile,
    _runtime: &'owner ChannelRuntimeV1,
    _objects: WorldStaticOwnerBorrow<'owner, O>,
}
impl<O: NativeWorldObjectRead + ?Sized> QualifiedSourceStepProbe<'_, O> {
    pub fn qualified_tile(&self) -> &crate::content::QualifiedSpellTile {
        self.tile
    }
}
pub async fn qualified_source_step_probe_in_transaction<
    'owner,
    O: NativeWorldObjectRead + ?Sized,
>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    read: &crate::durability::spell_item_transaction::StandingTileRead,
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &'owner ChannelRuntimeV1,
    objects: &'owner O,
    position: MovementLocalPosition,
) -> Result<QualifiedSourceStepProbe<'owner, O>, NativeWorldReadError<O::Error>> {
    read.check_transaction(tx)
        .await
        .map_err(NativeWorldReadError::Items)?;
    if read.fence().runtime_scope
        != crate::foundation::RuntimeScopeRefV1::channel(
            runtime.binding().world_id(),
            runtime.binding().channel_id(),
        )
        || read.fence().scope_ownership_generation != runtime.binding().scope_generation()
        || read.content_digest() != runtime.content_pin().server_artifact_digest()
        || room.source_world().is_none()
    {
        return Err(NativeWorldReadError::AuthorityMismatch);
    }
    let target = crate::durability::native_world_items::SpellGroundTarget::for_native_tile_read(
        room, runtime, position,
    )
    .map_err(NativeWorldReadError::GroundAddress)?;
    if read.target() != &target {
        return Err(NativeWorldReadError::AuthorityMismatch);
    }
    let owner = qualified_static_owner(room, runtime, objects)?;
    let tile = initialized_tile_in_transaction::<O::Error>(tx, room, runtime, position)
        .await?
        .ok_or(NativeWorldReadError::MissingTileMetadata)?;
    // Preserve solidity for the unchanged source algorithm. It rejects a solid
    // final destination, but legitimately queries solid neighbour flags first.
    Ok(QualifiedSourceStepProbe {
        tile,
        _runtime: runtime,
        _objects: owner,
    })
}

pub async fn qualified_combat_tile_in_transaction<'owner, O: NativeWorldObjectRead + ?Sized>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &crate::durability::spell_item_transaction::SpellItemAuthority,
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &'owner ChannelRuntimeV1,
    objects: &'owner O,
    position: MovementLocalPosition,
) -> Result<QualifiedCombatTileFact<'owner, O>, NativeWorldReadError<O::Error>> {
    let owner = qualified_static_owner(room, runtime, objects)?;
    let tile = tile_in_transaction::<O::Error>(tx, authority, room, runtime, position)
        .await?
        .ok_or(NativeWorldReadError::MissingTileMetadata)?;
    if owner.blocks_movement(position) {
        return Err(NativeWorldReadError::MissingDynamicProjectileFlags);
    }
    Ok(QualifiedCombatTileFact {
        position,
        tile,
        _runtime: runtime,
        _objects: owner,
    })
}

fn offset<E>(
    source: MovementLocalPosition,
    x: i32,
    y: i32,
    floor: i16,
) -> Result<MovementLocalPosition, NativeWorldReadError<E>> {
    Ok(MovementLocalPosition {
        x: source
            .x
            .checked_add(x)
            .ok_or(NativeWorldReadError::CoordinateOverflow)?,
        y: source
            .y
            .checked_add(y)
            .ok_or(NativeWorldReadError::CoordinateOverflow)?,
        floor: source
            .floor
            .checked_add(floor)
            .ok_or(NativeWorldReadError::CoordinateOverflow)?,
    })
}

pub fn source_sight_steps(
    mut a: MovementLocalPosition,
    mut b: MovementLocalPosition,
) -> Result<Vec<(MovementLocalPosition, bool)>, NativeWorldReadError<std::convert::Infallible>> {
    if a.floor != b.floor {
        return Err(NativeWorldReadError::MissingTileMetadata);
    }
    let dx = a.x.abs_diff(b.x);
    let dy = a.y.abs_diff(b.y);
    if dx.max(dy) > 64 {
        return Err(NativeWorldReadError::MissingTileMetadata);
    }
    let mut out = Vec::new();
    if dx.max(dy) <= 1 {
        return Ok(out);
    }
    if dx == 0 || dy == 0 {
        let (sx, sy) = ((b.x - a.x).signum(), (b.y - a.y).signum());
        for _ in 1..dx.max(dy) {
            a = offset(a, sx, sy, 0)?;
            out.push((a, false));
        }
        return Ok(out);
    }
    let y_major = dy > dx;
    if (y_major && a.y > b.y) || (!y_major && a.x > b.x) {
        std::mem::swap(&mut a, &mut b);
    }
    let minor_sign = if y_major {
        (b.x - a.x).signum()
    } else {
        (b.y - a.y).signum()
    };
    let adj =
        (((if y_major { dx } else { dy }) as u64) << 16) / (if y_major { dy } else { dx }) as u64;
    let adj = adj as u16;
    let mut acc = if minor_sign < 0 {
        0u16.wrapping_sub(adj)
    } else {
        0
    };
    for _ in 1..dx.max(dy) {
        let before = acc;
        acc = acc.wrapping_add(adj);
        let minor = if acc <= before { minor_sign } else { 0 };
        let near = a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1;
        a = if y_major {
            offset(a, minor, 1, 0)?
        } else {
            offset(a, 1, minor, 0)?
        };
        out.push((a, near));
    }
    Ok(out)
}
/// Creature committed-cause uses the same source Map/bootstrap proof and static
/// WorldObject borrow as Player combat, with independently current scope authority.
pub async fn qualified_creature_source_tile_in_transaction<
    'owner,
    O: NativeWorldObjectRead + ?Sized,
>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &crate::durability::spell_item_transaction::SpellItemScopeAuthority,
    room: &'owner crate::content::QualifiedNativeEntryRoom,
    runtime: &'owner ChannelRuntimeV1,
    objects: &'owner O,
    position: MovementLocalPosition,
) -> Result<QualifiedCombatTileFact<'owner, O>, NativeWorldReadError<O::Error>> {
    crate::durability::spell_item_transaction::check_scope_transaction(tx, authority)
        .await
        .map_err(NativeWorldReadError::Items)?;
    if authority.world_bytes() != *runtime.binding().world_id().as_bytes()
        || authority.channel_bytes() != *runtime.binding().channel_id().as_bytes()
        || authority.generation() != runtime.binding().scope_generation().get()
    {
        return Err(NativeWorldReadError::AuthorityMismatch);
    }
    let owner = qualified_static_owner(room, runtime, objects)?;
    let tile = initialized_tile_in_transaction::<O::Error>(tx, room, runtime, position)
        .await?
        .ok_or(NativeWorldReadError::MissingTileMetadata)?;
    if owner.blocks_movement(position) {
        return Err(NativeWorldReadError::MissingDynamicProjectileFlags);
    }
    Ok(QualifiedCombatTileFact {
        position,
        tile,
        _runtime: runtime,
        _objects: owner,
    })
}

fn cell(position: MovementLocalPosition) -> LogicalCell {
    LogicalCell {
        x: position.x,
        y: position.y,
        z: i32::from(position.floor),
    }
}
