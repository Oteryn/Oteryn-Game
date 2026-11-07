//! Network-coordinate composition, using the real sealed Source room metadata.
use super::*;
use oteryn_protocol_oteryn::actor_spell::{SpellCastIntent, SpellTarget, SpellTargetPosition};

struct SourcePositionAuthority {
    wire: SpellTargetPosition,
    physical: SpellTargetPosition,
    received: RefCell<Option<SpellCastIntent>>,
}
impl FreshAdmissionAuthority for SourcePositionAuthority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }
    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        None
    }
    async fn physical_native_cast_position(
        &self,
        position: SpellTargetPosition,
    ) -> Option<SpellTargetPosition> {
        // The oracle is the two retained Source metadata positions, not a
        // second implementation of the production coordinate arithmetic.
        (position == self.wire).then_some(self.physical)
    }
    async fn cast_spell_completion(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
        _: u64,
        intent: SpellCastIntent,
    ) -> Option<SpellCastOutcome> {
        *self.received.borrow_mut() = Some(intent);
        Some(SpellCastOutcome::rejected())
    }
}

#[test]
fn srx3_source_native_cast_network_boundary_maps_position_and_keeps_wire_binding()
-> Result<(), Box<dyn Error>> {
    run(async {
        let scene = crate::gameplay_transport::wire_position::test_scene::source_scene()?;
        assert!(scene.room.pinned_activity_source_room().is_some());
        assert_eq!(
            scene
                .runtime
                .read_actor_position(scene.actor)
                .map_err(|error| format!("{error:?}"))?
                .position(),
            scene.physical
        );
        let wire = SpellTargetPosition {
            x: scene.wire.x,
            y: scene.wire.y,
            floor: scene.wire.floor,
        };
        let physical = SpellTargetPosition {
            x: scene.physical.x,
            y: scene.physical.y,
            floor: scene.physical.floor,
        };
        let authority = SourcePositionAuthority {
            wire,
            physical,
            received: RefCell::new(None),
        };
        let intent = SpellCastIntent {
            spell: std::num::NonZeroU32::new(1).ok_or("spell")?,
            target: SpellTarget::Position(wire),
            aim_at_target: false,
        };
        let payload = oteryn_protocol_oteryn::actor_spell::encode_spell_cast_intent(&intent);
        let original = payload.clone();
        let admitted = AdmittedSession {
            game_session_id: scene.session,
            world_id: scene.runtime.content_pin().world_id(),
            channel_id: ChannelId::decode(&uuid_v7(2))?,
            runtime_actor: Some(scene.actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: Some(scene.fence),
        };
        let result = super::super::routing::spell::route(
            &authority,
            scene.actor,
            &admitted,
            oteryn_protocol_oteryn::ClientCommandView {
                command_id: 1,
                command_type: 3,
                payload: &payload,
            },
        )
        .await;
        assert!(
            result.is_some(),
            "actual decoded command reaches the existing cast dispatcher"
        );
        assert_eq!(
            authority
                .received
                .borrow()
                .as_ref()
                .map(|value| value.target),
            Some(SpellTarget::Position(physical)),
            "network native coordinates become the current Source runtime's physical coordinates exactly once"
        );
        assert_eq!(
            payload, original,
            "continuity's original wire payload remains unchanged"
        );
        assert_eq!(
            oteryn_protocol_oteryn::actor_spell::decode_spell_cast_intent(&payload)
                .map_err(|e| format!("{e:?}"))?,
            intent
        );
        Ok(())
    })
}
