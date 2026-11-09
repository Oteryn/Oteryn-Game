//! Owned read DATA. The caller keeps the transaction open through synchronous
//! revalidation/publication; these values never grant movement or Item mutation.
use super::*;
use crate::durability::native_world_items::SpellGroundTarget;
use crate::durability::spell_item_transaction::{
    SpellItemScopeAuthority, check_scope_transaction, read_creature_scope_tile_in_transaction,
};
use crate::durability::spell_items_abi::DurableTileItems;

#[derive(Clone)]
pub struct PreparedCreatureTileRead {
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: u64,
    activation: u64,
    client: [u8; 32],
    position: MovementLocalPosition,
    target: SpellGroundTarget,
    tile: Option<crate::content::QualifiedSpellTile>,
}

/// Unknown mutable stacks still require their initialization receipt owner.
/// They are never represented by a proved-empty cell in this detached path.
pub fn prepare_creature_tile_read<O: NativeWorldObjectRead + ?Sized>(
    room: &crate::content::QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    objects: &O,
    position: MovementLocalPosition,
) -> Result<PreparedCreatureTileRead, NativeWorldReadError<O::Error>> {
    let owner = qualified_static_owner(room, runtime, objects)?;
    if matches!(owner, WorldStaticOwnerBorrow::Entry(_)) && owner.blocks_movement(position) {
        return Err(NativeWorldReadError::MissingDynamicProjectileFlags);
    }
    let target = SpellGroundTarget::for_native_tile_read(room, runtime, position)
        .map_err(NativeWorldReadError::GroundAddress)?;
    Ok(PreparedCreatureTileRead {
        scope: runtime.binding().runtime_scope(),
        generation: runtime.binding().scope_generation().get(),
        activation: runtime.content_pin().activation_sequence(),
        client: runtime.content_pin().client_artifact_digest(),
        position,
        target,
        tile: tile::<O::Error>(room, position)?.map(std::borrow::Cow::into_owned),
    })
}
impl PreparedCreatureTileRead {
    pub fn current_for<O: NativeWorldObjectRead + ?Sized>(
        &self,
        room: &crate::content::QualifiedNativeEntryRoom,
        runtime: &ChannelRuntimeV1,
        objects: &O,
    ) -> bool {
        prepare_creature_tile_read(room, runtime, objects, self.position).is_ok_and(|now| {
            self.scope == now.scope
                && self.generation == now.generation
                && self.activation == now.activation
                && self.client == now.client
                && self.target == now.target
                && self.tile == now.tile
        })
    }
    pub fn position(&self) -> MovementLocalPosition {
        self.position
    }
}

pub struct ReadCreatureTile {
    prepared: PreparedCreatureTileRead,
    items: DurableTileItems,
}
impl ReadCreatureTile {
    pub fn prepared(&self) -> &PreparedCreatureTileRead {
        &self.prepared
    }
    pub fn qualified_tile(&self) -> Option<&crate::content::QualifiedSpellTile> {
        self.prepared.tile.as_ref()
    }
    pub fn items(&self) -> &DurableTileItems {
        &self.items
    }
}

/// No Runtime or LocalObject borrow crosses either SQL await. The independent
/// scope fence and actual scoped Item row locks remain the existing reader's.
pub async fn read_prepared_creature_tile_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &SpellItemScopeAuthority,
    prepared: &PreparedCreatureTileRead,
) -> Result<ReadCreatureTile, NativeWorldReadError<std::convert::Infallible>> {
    if authority.runtime_scope() != prepared.scope || authority.generation() != prepared.generation
    {
        return Err(NativeWorldReadError::AuthorityMismatch);
    }
    check_scope_transaction(tx, authority)
        .await
        .map_err(NativeWorldReadError::Items)?;
    let items = read_creature_scope_tile_in_transaction(tx, authority, &prepared.target)
        .await
        .map_err(NativeWorldReadError::Items)?;
    if prepared.tile.is_none() && !items.items.is_empty() {
        return Err(NativeWorldReadError::MissingTileMetadata);
    }
    check_scope_transaction(tx, authority)
        .await
        .map_err(NativeWorldReadError::Items)?;
    Ok(ReadCreatureTile {
        prepared: prepared.clone(),
        items,
    })
}
