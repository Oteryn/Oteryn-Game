//! Project one proven physical position into the existing wire position fields.
//! Qualified Bundles use native -z on their map wire and legacy +z in physics.
use crate::content::{NativeEntryMovementCells, QualifiedNativeEntryRoom};
use crate::foundation::{ChannelContentPin, MovementLocalPosition};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;

#[derive(Clone, Copy)]
pub(super) struct WirePositionFrame {
    native_source_bundle: bool,
}
impl WirePositionFrame {
    pub(super) fn from_pin(
        room: Option<&QualifiedNativeEntryRoom>,
        cells: &NativeEntryMovementCells,
        pin: &ChannelContentPin,
        map: [u8; 32],
    ) -> Option<Self> {
        let native_source_bundle = room.is_some()
            && matches!(
                cells.index(),
                crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
            );
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
        Some(Self {
            native_source_bundle,
        })
    }
    pub(super) fn project(self, physical: MovementLocalPosition) -> Option<ActorPosition> {
        let floor = if self.native_source_bundle {
            if !(0..=15).contains(&physical.floor) {
                return None;
            }
            physical.floor.checked_neg()?
        } else {
            physical.floor
        };
        Some(ActorPosition {
            x: physical.x,
            y: physical.y,
            floor,
        })
    }
    /// Coordinate DATA only; callers retain the original wire command and
    /// still apply every existing actor, map, cost and gameplay gate.
    pub(super) fn unproject(self, wire: ActorPosition) -> Option<MovementLocalPosition> {
        let floor = if self.native_source_bundle {
            if !(-15..=0).contains(&wire.floor) {
                return None;
            }
            wire.floor.checked_neg()?
        } else {
            wire.floor
        };
        Some(MovementLocalPosition {
            x: wire.x,
            y: wire.y,
            floor,
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
        let source_entry = source.entry_start();
        let wire = ActorPosition {
            x: source_entry.x,
            y: source_entry.y,
            floor: source_entry.floor,
        };
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
                floor: -8
            }),
            "domain1/entities/step wire must match native domain17"
        );
        assert_eq!(
            frame.unproject(ActorPosition {
                x: 32373,
                y: 31151,
                floor: -8
            }),
            Some(physical)
        );
        assert!(
            frame
                .unproject(ActorPosition {
                    x: 32373,
                    y: 31151,
                    floor: 8
                })
                .is_none(),
            "Source wire input uses the native floor convention"
        );
        let generic = source
            .bundle_view(
                bundle.movement_cells().clone(),
                bundle.frame_binding().digest(),
                bundle.map_revision_digest(),
                bundle.entry_start(),
            )
            .ok_or("generic view")?;
        assert_eq!(
            WirePositionFrame::from_pin(
                Some(&generic),
                generic.movement_cells(),
                &pin,
                generic.map_revision_digest()
            )
            .ok_or("legacy wire frame")?
            .project(physical),
            Some(ActorPosition {
                x: 32373,
                y: 31151,
                floor: -8
            }),
            "whole World Bundle positions must match its native map floor too"
        );
        assert!(
            WirePositionFrame::from_pin(Some(&bundle), bundle.movement_cells(), &pin, [0; 32])
                .is_none(),
            "a foreign map cannot project a sealed source frame"
        );
        Ok(())
    }
}
