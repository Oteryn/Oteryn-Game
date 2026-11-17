//! Resolve only identities carried by the current visible scene.
use crate::play::{GroundItem, PlayView};
use oteryn_client_runtime::world_interaction::{Target, world_object::WorldObjectTarget};
use oteryn_renderer::TileCoord;
use oteryn_session::world_map::MapOrigin;
impl PlayView {
    pub(in crate::play) fn world_context_target(&self, tile: TileCoord) -> Option<Target> {
        // NPCs and players carry look identities even though primary clicks cannot attack them.
        if let Some(entity) = self.entities.iter().rev().find(|entity| {
            entity.position.floor == self.floor
                && entity.position.x == tile.x
                && entity.position.y == tile.y
                && matches!(
                    entity.detail,
                    oteryn_session::EntityDetail::Actor { .. }
                        | oteryn_session::EntityDetail::ActorWithIcons { .. }
                )
        }) {
            return Some(Target::Creature(entity.entity));
        }
        let target = self.scene.target_at(tile)?;
        match target.item {
            Some(GroundItem::Map {
                header,
                position,
                item,
            }) => match item.origin {
                MapOrigin::Handle(handle) => Some(Target::Item(handle)),
                MapOrigin::BaseOrdinal {
                    ordinal,
                    object_revision,
                } => {
                    let x = u16::try_from(position.x).ok()?;
                    let y = u16::try_from(position.y).ok()?;
                    let z = u8::try_from(position.floor.checked_neg()?).ok()?;
                    let key = (u64::from(x) << 32)
                        | (u64::from(y) << 16)
                        | (u64::from(z) << 8)
                        | u64::from(ordinal);
                    let mut placement = header.bundle_digest.to_vec();
                    placement.extend(key.to_be_bytes());
                    Some(Target::Object(WorldObjectTarget {
                        placement,
                        expected_revision: object_revision,
                    }))
                }
                MapOrigin::DisplayOnly => None,
            },
            Some(GroundItem::Entity { handle, .. }) => handle.map(Target::Item),
            None => target.entity.map(Target::Creature),
        }
    }
}
