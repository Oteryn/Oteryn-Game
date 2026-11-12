//! W07 child map qualification under the currently activated World content.
//! Crystal 201189f data-global/lib/others/soulpit.lua:31-84 defines this bounded
//! arena; data-global/scripts/quests/soulpit/soulpit_fight.lua:37-77 defines entry.

use crate::content::channel_content_pin::ChannelContentPin;
use crate::content::{
    ActiveGeneration, CollisionClass, ContentActivationController, LogicalCell,
    QualifiedNativeEntryRoom,
};

pub const SOULPIT_TEMPLATE_KEY: &str = "oteryn:room.soulpit";
pub const SOULPIT_TEMPLATE_REVISION: &str = "crystal-201189f-soulpit-r1";
// MovementLocalPosition and Bundle collision retain source-z=8; map TilePos
// projects this once to -8 (bundle_spell_tiles::tile_pos / bundle_map_position).
pub const SOULPIT_BOUNDS: [i32; 6] = [32362, 31132, 32390, 31162, 8, 8];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoulpitTemplateRefusal {
    NoActiveGeneration,
    ContentChanged,
    SourceMapUnavailable,
    UnknownPlacement,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::bundle_spell_tiles::{BundleSpellTiles, BundleTileBase};
    use crate::content::native_cell_lookup::{BundleCollisionIndex, BundleCollisionLookup};
    use crate::content::static_cell_engine::{EngineeringStaticCellScope, StaticCellEngineError};
    use std::sync::Arc;

    #[derive(Debug)]
    struct TestBundle {
        scope: EngineeringStaticCellScope,
        blocked_entry: bool,
    }
    impl BundleCollisionLookup for TestBundle {
        fn lookup(
            &self,
            scope: &EngineeringStaticCellScope,
            cell: LogicalCell,
        ) -> Result<CollisionClass, StaticCellEngineError> {
            if scope != &self.scope {
                return Err(StaticCellEngineError::ScopeMismatch);
            }
            if self.blocked_entry && cell.x == 32374 && cell.y == 31151 && cell.z == 8 {
                return Ok(CollisionClass::Blocked);
            }
            Ok(CollisionClass::Walkable)
        }
        fn same_index(&self, other: &dyn BundleCollisionLookup) -> bool {
            (other as &dyn std::any::Any)
                .downcast_ref::<Self>()
                .is_some_and(|other| {
                    self.scope == other.scope && self.blocked_entry == other.blocked_entry
                })
        }
    }
    impl BundleTileBase for TestBundle {
        fn digest(&self) -> [u8; 32] {
            [19; 32]
        }
        fn tile_index(&self, _: u16, _: u16, _: i8) -> Option<usize> {
            None
        }
    }

    fn fixture(
        blocked_entry: bool,
    ) -> Result<(ContentActivationController, QualifiedNativeEntryRoom), String> {
        let mut id = [29; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let world = oteryn_protocol_oteryn::WorldId::decode(&id).map_err(|e| format!("{e:?}"))?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let room = pin.qualified_room();
        let mut scope = room.movement_cells().scope().clone();
        scope.coordinate_frame =
            crate::content::CoordinateFrameRef::new("oteryn:frame/world-bundle-v3")
                .map_err(|e| format!("{e:?}"))?;
        let source = Arc::new(TestBundle {
            scope: scope.clone(),
            blocked_entry,
        });
        let cells = crate::content::NativeEntryMovementCells::from_bundle(
            room.movement_cells(),
            scope,
            BundleCollisionIndex(source.clone()),
            Arc::new(BundleSpellTiles {
                base: source,
                tiles: vec![],
                shapes: vec![],
            }),
        );
        let view = room
            .bundle_view(
                cells,
                [13; 32],
                [14; 32],
                crate::content::NativeEntryStart {
                    x: 32373,
                    y: 31151,
                    floor: 8,
                },
            )
            .ok_or("test Bundle view")?;
        Ok((controller, view))
    }

    #[test]
    fn soulpit_qualified_bundle_view_preserves_pins_and_clips_origin_cells() -> Result<(), String> {
        let (controller, room) = fixture(false)?;
        let template = controller
            .qualify_soulpit_activity_template(&room, "content-r1")
            .map_err(|e| format!("{e:?}"))?;
        let cells = template.room().movement_cells();
        assert_eq!(
            cells.index().lookup(
                cells.scope(),
                LogicalCell {
                    x: 32362,
                    y: 31132,
                    z: 8,
                },
            ),
            Ok(CollisionClass::Walkable)
        );
        for cell in [
            LogicalCell {
                x: 32361,
                y: 31132,
                z: 8,
            },
            LogicalCell {
                x: 32362,
                y: 31163,
                z: 8,
            },
            LogicalCell {
                x: 32362,
                y: 31132,
                z: 7,
            },
        ] {
            assert_eq!(
                cells.index().lookup(cells.scope(), cell),
                Err(StaticCellEngineError::Absent)
            );
        }
        let active = controller.active().ok_or("active content")?;
        let pin = template
            .issue_runtime_pin(active)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(pin.frame_binding_digest(), room.frame_binding().digest());
        assert_eq!(pin.map_revision_digest(), room.map_revision_digest());
        assert_eq!(
            pin.server_artifact_digest(),
            active.identity().server_artifact_digest()
        );
        assert_eq!(
            pin.client_artifact_digest(),
            active.identity().client_artifact_digest()
        );
        assert_eq!(
            (
                pin.entry_start().x,
                pin.entry_start().y,
                pin.entry_start().floor
            ),
            (32373, 31151, 8)
        );
        Ok(())
    }

    #[test]
    fn soulpit_refuses_a_blocked_entry_without_changing_parent_bundle() -> Result<(), String> {
        let (controller, room) = fixture(true)?;
        let before = room.clone();
        assert!(matches!(
            controller.qualify_soulpit_activity_template(&room, "content-r1"),
            Err(SoulpitTemplateRefusal::UnknownPlacement)
        ));
        assert_eq!(room, before);
        Ok(())
    }
}

/// Only the Content activation owner constructs this view. It retains its
/// parent's map/frame/pair identities and cannot grant an Instance assignment.
#[derive(Debug)]
pub struct QualifiedSoulpitActivityTemplate {
    room: QualifiedNativeEntryRoom,
    content_revision: String,
    activation_sequence: u64,
    server_digest: [u8; 32],
    client_digest: [u8; 32],
}

impl QualifiedSoulpitActivityTemplate {
    pub fn room(&self) -> &QualifiedNativeEntryRoom {
        &self.room
    }
    pub const fn template_key(&self) -> &str {
        SOULPIT_TEMPLATE_KEY
    }
    pub const fn template_revision(&self) -> &str {
        SOULPIT_TEMPLATE_REVISION
    }
    pub fn content_revision(&self) -> &str {
        &self.content_revision
    }
    pub fn staging_positions(&self) -> [LogicalCell; 5] {
        std::array::from_fn(|slot| LogicalCell {
            x: 32375,
            y: 31158 + slot as i32,
            z: 8,
        })
    }
    pub fn entry_positions(&self) -> [LogicalCell; 5] {
        std::array::from_fn(|slot| LogicalCell {
            x: 32373 + slot as i32,
            y: 31151,
            z: 8,
        })
    }
    pub const fn exit_position(&self) -> LogicalCell {
        LogicalCell {
            x: 32373,
            y: 31158,
            z: 8,
        }
    }
    pub fn issue_runtime_pin(
        &self,
        active: &ActiveGeneration,
    ) -> Result<ChannelContentPin, SoulpitTemplateRefusal> {
        if active.activation_sequence() != self.activation_sequence
            || active.identity().server_artifact_digest() != self.server_digest
            || active.identity().client_artifact_digest() != self.client_digest
        {
            return Err(SoulpitTemplateRefusal::ContentChanged);
        }
        let start = self.room.entry_start();
        Ok(ChannelContentPin::from_activation(
            self.room.movement_cells().scope().world_id,
            self.activation_sequence,
            self.server_digest,
            self.client_digest,
            self.room.frame_binding().digest(),
            self.room.map_revision_digest(),
            (start.x, start.y, start.floor),
        ))
    }
}

impl ContentActivationController {
    pub fn qualify_soulpit_activity_template(
        &self,
        room: &QualifiedNativeEntryRoom,
        content_revision: &str,
    ) -> Result<QualifiedSoulpitActivityTemplate, SoulpitTemplateRefusal> {
        let active = self
            .active()
            .filter(|_| self.is_ready())
            .ok_or(SoulpitTemplateRefusal::NoActiveGeneration)?;
        if content_revision.is_empty()
            || content_revision.len() > 128
            || room.compiled().server_digest() != active.identity().server_artifact_digest()
            || room.compiled().client_digest() != active.identity().client_artifact_digest()
        {
            return Err(SoulpitTemplateRefusal::ContentChanged);
        }
        // Soulpit's donor coordinates are the full World bundle's absolute
        // native frame. A translated engineering/Thalom source cannot qualify.
        let room = room
            .bounded_bundle_activity_view(
                SOULPIT_BOUNDS,
                crate::content::NativeEntryStart {
                    x: 32373,
                    y: 31151,
                    floor: 8,
                },
            )
            .ok_or(SoulpitTemplateRefusal::SourceMapUnavailable)?;
        let template = QualifiedSoulpitActivityTemplate {
            room,
            content_revision: content_revision.into(),
            activation_sequence: active.activation_sequence(),
            server_digest: active.identity().server_artifact_digest(),
            client_digest: active.identity().client_artifact_digest(),
        };
        for cell in template
            .staging_positions()
            .into_iter()
            .chain(template.entry_positions())
            .chain([
                template.exit_position(),
                LogicalCell {
                    x: 32376,
                    y: 31144,
                    z: 8,
                },
            ])
        {
            let cells = template.room.movement_cells();
            if cells.index().lookup(cells.scope(), cell).ok() != Some(CollisionClass::Walkable) {
                return Err(SoulpitTemplateRefusal::UnknownPlacement);
            }
        }
        Ok(template)
    }
}
