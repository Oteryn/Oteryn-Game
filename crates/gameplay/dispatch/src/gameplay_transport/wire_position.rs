//! Project one proven physical position into the existing wire position fields.
//! Wire floors use Global z, with ground at 7. Older source and entry indices use signed floors.
use crate::content::{
    NativeEntryMovementCells, QualifiedNativeEntryRoom,
    native_cell_lookup::NativeMovementCollisionIndex,
};
use crate::foundation::{ChannelContentPin, MovementLocalPosition};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;

#[derive(Clone, Copy)]
pub(super) struct WirePositionFrame {
    signed_floor: bool,
}
impl WirePositionFrame {
    pub(super) fn from_pin(
        room: Option<&QualifiedNativeEntryRoom>,
        cells: &NativeEntryMovementCells,
        pin: &ChannelContentPin,
        map: [u8; 32],
    ) -> Option<Self> {
        let bundle = matches!(cells.index(), NativeMovementCollisionIndex::Bundle(_));
        let native_source_bundle =
            bundle && room.is_some_and(|r| r.pinned_activity_source_room().is_some());
        if native_source_bundle {
            let room = room?;
            if cells.scope() != room.movement_cells().scope()
                || cells.index() != room.movement_cells().index()
                || pin.world_id() != room.movement_cells().scope().world_id
                || pin.server_artifact_digest() != room.compiled().server_digest()
                || pin.client_artifact_digest() != room.compiled().client_digest()
                || pin.frame_binding_digest() != room.frame_binding().digest()
                || map != room.map_revision_digest()
            {
                return None;
            }
        }
        // The bundle collision owner uses Global z. The qualified legacy frame
        // or the runtime's pinned start identifies the signed source convention.
        let signed_floor = match cells.index() {
            NativeMovementCollisionIndex::Bundle(_) => false,
            NativeMovementCollisionIndex::Source(index) => index.entry_start().floor < 0,
            NativeMovementCollisionIndex::Entry(_) => pin.entry_start().floor < 0,
        };
        Some(Self { signed_floor })
    }
    pub(super) fn project(self, physical: MovementLocalPosition) -> Option<ActorPosition> {
        let floor = if self.signed_floor {
            crate::movement::interest::tibia_floor(physical.floor)?
        } else {
            (0..=15)
                .contains(&physical.floor)
                .then_some(physical.floor)?
        };
        Some(ActorPosition {
            x: physical.x,
            y: physical.y,
            floor,
        })
    }
    pub(super) fn project_entity(
        self,
        mut entity: super::world_spatial::ChannelEntity,
    ) -> Option<super::world_spatial::ChannelEntity> {
        let ActorPosition { x, y, floor } = entity.position;
        entity.position = self.project(MovementLocalPosition { x, y, floor })?;
        Some(entity)
    }

    /// Coordinate DATA only; callers retain the original wire command and
    /// still apply every existing actor, map, cost and gameplay gate.
    pub(super) fn unproject(self, wire: ActorPosition) -> Option<MovementLocalPosition> {
        if !(0..=15).contains(&wire.floor) {
            return None;
        }
        Some(MovementLocalPosition {
            x: wire.x,
            y: wire.y,
            floor: if self.signed_floor {
                -wire.floor
            } else {
                wire.floor
            },
        })
    }
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn wire_position_frame(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
    ) -> Option<WirePositionFrame> {
        WirePositionFrame::from_pin(
            self.effective_rooms().gate_wire_position(),
            self.effective_movement_cells(),
            runtime.content_pin(),
            runtime.content_pin().map_revision_digest(),
        )
    }
}

#[cfg(test)]
pub(in crate::gameplay_transport) mod test_scene {
    use super::*;
    pub struct SourceScene {
        pub controller: crate::content::ContentActivationController,
        pub room: QualifiedNativeEntryRoom,
        pub capture_room: QualifiedNativeEntryRoom,
        pub runtime: crate::foundation::ChannelRuntimeV1,
        pub actor: crate::foundation::ExactActorRef,
        pub session: crate::foundation::GameSessionId,
        pub fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        pub wire: ActorPosition,
        pub physical: MovementLocalPosition,
        pub book: crate::spell::SpellBook,
    }
    pub fn source_scene() -> Result<SourceScene, String> {
        use crate::foundation::*;
        let id = |n| {
            let mut b = [n; 16];
            b[6] = 0x70;
            b[8] = 0x80;
            b
        };
        fn err(value: impl std::fmt::Debug) -> String {
            format!("{value:?}")
        }
        let world = WorldId::decode(&id(1)).map_err(err)?;
        let channel = ChannelId::decode(&id(2)).map_err(err)?;
        let (controller, base) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active Source content")?;
        let book = active
            .native_gameplay()
            .ok_or("active native spell book")?
            .spell_book()
            .clone();
        let kind = crate::content::bosses::activity_template::PinnedActivitySourceRoom::Soulpit;
        let source = active
            .qualify_activity_source_room(base.qualified_room(), kind)
            .map_err(err)?;
        let room = crate::gameplay_transport::activity_runtime::source_bundle::qualify(
            &source, kind, active, world, channel,
        )
        .map_err(|_| "actual Source Bundle")?;
        let at = room.entry_start();
        let physical = MovementLocalPosition {
            x: at.x,
            y: at.y,
            floor: at.floor,
        };
        let captured = source.entry_start();
        let wire = ActorPosition {
            x: captured.x,
            y: captured.y,
            floor: captured.floor,
        };
        let pin = ChannelContentPin::from_activation(
            world,
            active.activation_sequence(),
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (at.x, at.y, at.floor),
        );
        let scope = RuntimeScopeRefV1::instance(world, id(3)).map_err(err)?;
        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            scope,
            NodeId::decode(&id(4)).map_err(err)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(err)?;
        let session = GameSessionId::decode(&id(5)).map_err(err)?;
        let reserved = runtime.reserve_fresh_session(session).map_err(err)?;
        let actor = runtime.commit_fresh_session(reserved).map_err(err)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(err)?;
        let fence = crate::durability::item_transfer::CurrentCharacterItemFence {
            character_id: crate::domain::CharacterId::from_bytes(id(6)).map_err(err)?,
            game_session_id: session,
            connection_generation: ConnectionGeneration::new(1).map_err(err)?,
            character_lease_generation: 1,
            runtime_scope: scope,
            scope_ownership_generation: ScopeOwnershipGeneration::new(1).map_err(err)?,
        };
        Ok(SourceScene {
            controller,
            room,
            capture_room: source,
            runtime,
            actor,
            session,
            fence,
            wire,
            physical,
            book,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
    #[test]
    fn srx3_pinned_activity_wire_positions_match_native_map_floor() -> Result<(), String> {
        let id = |n| {
            let mut b = [n; 16];
            b[6] = 0x70;
            b[8] = 0x80;
            b
        };
        let world = crate::foundation::WorldId::decode(&id(1)).map_err(|e| format!("{e:?}"))?;
        let channel = crate::foundation::ChannelId::decode(&id(2)).map_err(|e| format!("{e:?}"))?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active")?;
        let source = active
            .qualify_activity_source_room(pin.qualified_room(), PinnedActivitySourceRoom::Soulpit)
            .map_err(|e| format!("{e:?}"))?;
        let bundle = crate::gameplay_transport::activity_runtime::source_bundle::qualify(
            &source,
            PinnedActivitySourceRoom::Soulpit,
            active,
            world,
            channel,
        )
        .map_err(|_| "actual source Bundle")?;
        let pin = ChannelContentPin::from_activation(
            world,
            active.activation_sequence(),
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            bundle.frame_binding().digest(),
            bundle.map_revision_digest(),
            (32373, 31151, 8),
        );
        let physical = MovementLocalPosition {
            x: 32373,
            y: 31151,
            floor: 8,
        };
        let frame = WirePositionFrame::from_pin(
            Some(&bundle),
            bundle.movement_cells(),
            &pin,
            bundle.map_revision_digest(),
        )
        .ok_or("source wire frame")?;
        assert_eq!(
            frame.project(physical),
            Some(ActorPosition {
                x: 32373,
                y: 31151,
                floor: 8
            }),
            "domain1/entities/step wire must match native domain17"
        );
        assert_eq!(
            frame.unproject(ActorPosition {
                x: 32373,
                y: 31151,
                floor: 8
            }),
            Some(physical)
        );
        assert!(
            frame
                .unproject(ActorPosition {
                    x: 32373,
                    y: 31151,
                    floor: -8
                })
                .is_none(),
            "Source wire input rejects a negative floor"
        );
        let generic = source
            .bundle_view(
                bundle.movement_cells().clone(),
                bundle.frame_binding().digest(),
                bundle.map_revision_digest(),
                bundle.entry_start(),
            )
            .ok_or("generic view")?;
        let legacy = WirePositionFrame::from_pin(
            Some(&generic),
            generic.movement_cells(),
            &pin,
            generic.map_revision_digest(),
        )
        .ok_or("legacy wire frame")?;
        assert_eq!(
            legacy.unproject(ActorPosition {
                x: 32373,
                y: 31151,
                floor: 8
            }),
            Some(physical),
            "an ordinary Bundle reads its raw +z wire floor back unchanged"
        );
        assert_eq!(
            legacy.project(physical),
            Some(ActorPosition {
                x: 32373,
                y: 31151,
                floor: 8
            }),
            "ordinary Bundle contract remains explicit"
        );
        assert!(
            WirePositionFrame::from_pin(Some(&bundle), bundle.movement_cells(), &pin, [0; 32])
                .is_none(),
            "a foreign map cannot project a sealed source frame"
        );
        Ok(())
    }
    #[test]
    fn source_and_ordinary_bundle_snapshots_share_the_physical_floor() -> Result<(), String> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(check_source_and_ordinary_bundle_snapshots)
            .map_err(|error| error.to_string())?
            .join()
            .map_err(|_| "snapshot scenario panicked".to_owned())?
    }

    fn check_source_and_ordinary_bundle_snapshots() -> Result<(), String> {
        use crate::gameplay_transport::world_spatial::{
            ChannelEntities, ChannelEntity, EntityLights, SessionVisibility, VisibleKind,
        };
        use std::num::NonZeroU32;

        let scene = super::test_scene::source_scene()?;
        let ordinary = scene
            .capture_room
            .bundle_view(
                scene.room.movement_cells().clone(),
                scene.room.frame_binding().digest(),
                scene.room.map_revision_digest(),
                scene.room.entry_start(),
            )
            .ok_or("ordinary Bundle view")?;
        assert!(ordinary.pinned_activity_source_room().is_none());
        let item_definition_ref = NonZeroU32::MIN;
        let kinds = [
            VisibleKind::Player,
            VisibleKind::Creature,
            VisibleKind::Corpse {
                item_definition_ref,
                native_appearance_id: None,
            },
            VisibleKind::GroundItem {
                item_definition_ref,
            },
        ];
        let at = scene.physical;
        assert_eq!(at.floor, 8);
        for room in [&scene.room, &ordinary] {
            let pin = scene.runtime.content_pin();
            let frame = WirePositionFrame::from_pin(
                Some(room),
                room.movement_cells(),
                pin,
                room.map_revision_digest(),
            )
            .ok_or("snapshot frame")?;
            let direct =
                crate::gameplay_transport::ComposedFreshAdmission::observation(&scene.runtime, at);
            let entities = kinds
                .iter()
                .enumerate()
                .map(|(index, &kind)| {
                    frame.project_entity(ChannelEntity {
                        source_icons: Default::default(),
                        motion: None,
                        display_name: None,
                        health_percent: 100,
                        direction: oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                        kind,
                        identity: [index as u8 + 1; 16],
                        generation: u64::from(index < 2),
                        position: direct.actor_position,
                        revision: 1,
                    })
                })
                .collect::<Option<Vec<_>>>()
                .ok_or("entity positions")?;
            let snapshot = SessionVisibility::with_objects(true)
                .snapshot(
                    &ChannelEntities {
                        content_generation: pin.client_artifact_digest(),
                        observer: [1; 16],
                        entities,
                        lights: EntityLights::new(),
                        observed_at_ms: 0,
                    },
                    &mut |_| Ok(()),
                )
                .map_err(|error| format!("{error:?}"))?;
            assert_eq!(snapshot.actor_position, direct.actor_position);
            assert_eq!(snapshot.entities.len(), kinds.len());
            for entity in snapshot.entities {
                assert_eq!(entity.position.floor, at.floor);
                assert!((0..=15).contains(&entity.position.floor));
            }
        }
        Ok(())
    }
}
