//! Soulpit's source-authored multi-wave room under the current content activation.
//! Crystal 201189f, data-global/lib/others/soulpit.lua:31-84 and
//! data-global/scripts/quests/soulpit/soulpit_fight.lua:37-110.
use super::activity_template::ActivityTemplateRefusal;
use crate::content::channel_content_pin::ChannelContentPin;
use crate::content::{ActiveGeneration, CollisionClass, LogicalCell, QualifiedNativeEntryRoom};

pub const TEMPLATE_KEY: &str = "oteryn:room.soulpit";
pub const ARENA_BOUNDS: [i32; 6] = [32362, 31132, 32390, 31153, 8, 8];
const CRYSTAL: &str = "201189f871d23597b82caf88fd611454a2ddefb8";

#[derive(Debug, Clone)]
pub struct QualifiedSoulpitRoomTemplate {
    room: QualifiedNativeEntryRoom,
    revision: String,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    entries: [LogicalCell; 5],
    staging: [LogicalCell; 5],
}

impl QualifiedSoulpitRoomTemplate {
    pub fn qualify(
        room: &QualifiedNativeEntryRoom,
        active: &ActiveGeneration,
    ) -> Result<Self, ActivityTemplateRefusal> {
        use ActivityTemplateRefusal as R;
        if room.compiled().server_digest() != active.identity().server_artifact_digest()
            || room.compiled().client_digest() != active.identity().client_artifact_digest()
            || room.movement_cells().scope().generation_digest
                != active.identity().server_artifact_digest()
            || !matches!(
                room.movement_cells().index(),
                crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
            )
            || active.native_gameplay().is_none()
        {
            return Err(R::NoActiveGeneration);
        }
        let entries = std::array::from_fn(|slot| LogicalCell {
            x: 32373 + slot as i32,
            y: 31151,
            z: 8,
        });
        let staging = std::array::from_fn(|slot| LogicalCell {
            x: 32375,
            y: 31158 + slot as i32,
            z: 8,
        });
        let cells = room.movement_cells();
        for at in entries
            .iter()
            .chain(staging.iter())
            .copied()
            .chain([Self::source_exit(), Self::source_boss()])
        {
            if cells.index().lookup(cells.scope(), at) != Ok(CollisionClass::Walkable)
                || cells.spell_tiles().lookup(cells.scope(), at).is_err()
            {
                return Err(R::UnknownPlacement);
            }
        }
        // The arena exit is a USE target; its own object may block walking.
        let portal = LogicalCell {
            x: 32375,
            y: 31153,
            z: 8,
        };
        if cells.index().lookup(cells.scope(), portal).is_err()
            || cells.spell_tiles().lookup(cells.scope(), portal).is_err()
        {
            return Err(R::UnknownPlacement);
        }
        let map: String = room
            .map_revision_digest()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let view = room
            .bundle_activity_view(ARENA_BOUNDS, entries[0])
            .ok_or(R::UnknownPlacement)?;
        Ok(Self {
            room: view,
            revision: format!("crystal-{CRYSTAL}-{map}"),
            activation: active.activation_sequence(),
            server: active.identity().server_artifact_digest(),
            client: active.identity().client_artifact_digest(),
            entries,
            staging,
        })
    }
    pub fn room(&self) -> &QualifiedNativeEntryRoom {
        &self.room
    }
    pub fn template_key(&self) -> &'static str {
        TEMPLATE_KEY
    }
    pub fn template_revision(&self) -> &str {
        &self.revision
    }
    pub fn entry_positions(&self) -> &[LogicalCell; 5] {
        &self.entries
    }
    pub fn staging_positions(&self) -> &[LogicalCell; 5] {
        &self.staging
    }
    pub fn exit_position(&self) -> LogicalCell {
        Self::source_exit()
    }
    pub fn boss_position(&self) -> LogicalCell {
        Self::source_boss()
    }
    fn source_exit() -> LogicalCell {
        LogicalCell {
            x: 32373,
            y: 31158,
            z: 8,
        }
    }
    fn source_boss() -> LogicalCell {
        LogicalCell {
            x: 32376,
            y: 31144,
            z: 8,
        }
    }
    pub fn issue_runtime_pin(
        &self,
        active: &ActiveGeneration,
    ) -> Result<ChannelContentPin, ActivityTemplateRefusal> {
        if active.activation_sequence() != self.activation
            || active.identity().server_artifact_digest() != self.server
            || active.identity().client_artifact_digest() != self.client
        {
            return Err(ActivityTemplateRefusal::NoActiveGeneration);
        }
        let start = self.entries[0];
        Ok(ChannelContentPin::from_activation(
            self.room.movement_cells().scope().world_id,
            self.activation,
            self.server,
            self.client,
            self.room.frame_binding().digest(),
            self.room.map_revision_digest(),
            (start.x, start.y, start.z as i16),
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "tests construct pinned, infallible source fixtures"
    )]
    use super::*;
    use crate::content::bundle_spell_tiles::{BundleSpellTiles, BundleTileBase};
    use crate::content::native_cell_lookup::{BundleCollisionIndex, BundleCollisionLookup};
    use crate::content::static_cell_engine::{EngineeringStaticCellScope, StaticCellEngineError};
    use crate::content::{
        NativeEntryMovementCells, NativeEntryStart, NativeSpellTileFlags, QualifiedSpellTile,
    };
    use std::sync::Arc;
    #[derive(Debug)]
    struct AllGround(EngineeringStaticCellScope);
    impl BundleCollisionLookup for AllGround {
        fn lookup(
            &self,
            scope: &EngineeringStaticCellScope,
            _: LogicalCell,
        ) -> Result<CollisionClass, StaticCellEngineError> {
            if scope != &self.0 {
                return Err(StaticCellEngineError::ScopeMismatch);
            }
            Ok(CollisionClass::Walkable)
        }
        fn same_index(&self, other: &dyn BundleCollisionLookup) -> bool {
            other.type_id() == std::any::TypeId::of::<Self>()
        }
    }
    impl BundleTileBase for AllGround {
        fn digest(&self) -> [u8; 32] {
            [71; 32]
        }
        fn tile_index(&self, _: u16, _: u16, _: i8) -> Option<usize> {
            Some(0)
        }
    }
    fn fixture() -> (
        crate::content::ContentActivationController,
        QualifiedNativeEntryRoom,
    ) {
        let mut id = [1; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let world = oteryn_protocol_oteryn::WorldId::decode(&id).unwrap();
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let room = pin.qualified_room();
        let mut scope = room.movement_cells().scope().clone();
        scope.coordinate_frame =
            crate::content::CoordinateFrameRef::new("oteryn:frame/soulpit-room").unwrap();
        let base = Arc::new(AllGround(scope.clone()));
        let spell = QualifiedSpellTile::from_bundle(
            Some((103, 150)),
            Vec::new(),
            NativeSpellTileFlags {
                block_solid: false,
                block_projectile: false,
                immovable_block_solid: false,
                immovable_block_item: false,
                immovable_nonfield_block_item: false,
                floor_change: false,
                protection_zone: false,
            },
            None,
        );
        let tiles = Arc::new(BundleSpellTiles {
            base: base.clone(),
            tiles: vec![0],
            shapes: vec![spell],
        });
        let cells = NativeEntryMovementCells::from_bundle(
            room.movement_cells(),
            scope,
            BundleCollisionIndex(base),
            tiles,
        );
        let room = room
            .bundle_view(
                cells,
                [71; 32],
                [72; 32],
                NativeEntryStart {
                    x: 32373,
                    y: 31151,
                    floor: 8,
                },
            )
            .unwrap();
        (controller, room)
    }
    #[test]
    fn soulpit_template_retained_pin_requires_exact_activation() {
        let (controller, room) = fixture();
        let active = controller.active().unwrap();
        let mut template = QualifiedSoulpitRoomTemplate::qualify(&room, active).unwrap();
        let pin = template.issue_runtime_pin(active).unwrap();
        assert_eq!(pin.frame_binding_digest(), room.frame_binding().digest());
        assert_eq!(pin.map_revision_digest(), room.map_revision_digest());
        assert_eq!(
            template.exit_position(),
            LogicalCell {
                x: 32373,
                y: 31158,
                z: 8
            }
        );
        assert_eq!(template.template_key(), TEMPLATE_KEY);
        template.activation += 1;
        assert_eq!(
            template.issue_runtime_pin(active),
            Err(ActivityTemplateRefusal::NoActiveGeneration)
        );
    }
    #[test]
    fn soulpit_template_bounds_collision_and_spell_tiles_without_origin_spawns() {
        let (controller, room) = fixture();
        let template =
            QualifiedSoulpitRoomTemplate::qualify(&room, controller.active().unwrap()).unwrap();
        let cells = template.room().movement_cells();
        let inside = template.entry_positions()[0];
        assert_eq!(
            cells.index().lookup(cells.scope(), inside),
            Ok(CollisionClass::Walkable)
        );
        assert!(cells.spell_tiles().lookup(cells.scope(), inside).is_ok());
        for outside in [
            LogicalCell {
                x: 32361,
                y: 31144,
                z: 8,
            },
            template.staging_positions()[0],
            LogicalCell { z: 7, ..inside },
        ] {
            assert!(
                cells.index().lookup(cells.scope(), outside).is_err(),
                "outside movement {outside:?}"
            );
            assert!(
                cells.spell_tiles().lookup(cells.scope(), outside).is_err(),
                "outside spell tile {outside:?}"
            );
        }
        assert!(template.room().spawn_source().is_none());
    }
}
