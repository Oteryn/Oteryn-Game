//! Native map presentation negotiation; it grants no cast or map mutation authority.
use super::capabilities::OfferedCapability;
use crate::content::{ActiveGeneration, QualifiedNativeEntryRoom};
use crate::foundation::{ChannelContentPin, ExactActorRef, GameSessionId, GameSessionState};

pub(super) fn source_available(
    active: &ActiveGeneration,
    room: &QualifiedNativeEntryRoom,
    pin: &ChannelContentPin,
) -> bool {
    matches!(
        room.movement_cells().index(),
        crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
    ) && active
        .native_gameplay()
        .is_some_and(|native| native.source_digest() == pin.server_artifact_digest())
        && active.identity().world_id() == pin.world_id()
        && active.activation_sequence() == pin.activation_sequence()
        && active.identity().server_artifact_digest() == pin.server_artifact_digest()
        && active.identity().client_artifact_digest() == pin.client_artifact_digest()
        && room.frame_binding().digest() == pin.frame_binding_digest()
        && room.map_revision_digest() == pin.map_revision_digest()
}

pub(super) fn offered(
    base: &[OfferedCapability],
    source_available: bool,
) -> Vec<OfferedCapability> {
    use oteryn_protocol_oteryn::presentation::{
        CAPABILITY_PRESENTATION_V1, CAPABILITY_PRESENTATION_V1_REQUIRES,
    };
    let mut result = base.to_vec();
    if source_available
        && !result
            .iter()
            .any(|offer| offer.id == CAPABILITY_PRESENTATION_V1)
    {
        let at = result.partition_point(|offer| offer.id < CAPABILITY_PRESENTATION_V1);
        result.insert(
            at,
            OfferedCapability {
                id: CAPABILITY_PRESENTATION_V1,
                requires: &CAPABILITY_PRESENTATION_V1_REQUIRES,
            },
        );
    }
    if source_available
        && result.iter().any(|offer| offer.id == 30)
        && !result.iter().any(|offer| {
            offer.id == oteryn_protocol_oteryn::native_map_cues::CAPABILITY_NATIVE_MAP_CUES_V1
        })
    {
        use oteryn_protocol_oteryn::native_map_cues::{
            CAPABILITY_NATIVE_MAP_CUES_V1, CAPABILITY_NATIVE_MAP_CUES_V1_REQUIRES,
        };
        let at = result.partition_point(|offer| offer.id < CAPABILITY_NATIVE_MAP_CUES_V1);
        result.insert(
            at,
            OfferedCapability {
                id: CAPABILITY_NATIVE_MAP_CUES_V1,
                requires: &CAPABILITY_NATIVE_MAP_CUES_V1_REQUIRES,
            },
        );
    }
    result
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn native_map_cue_source_frame(&self) -> Option<bool> {
        let active = self.active_generation?;
        let room = self.effective_rooms().gate_wire_position()?;
        let runtime = self.runtime.lock().await;
        (runtime.owner_fence().is_ok() && source_available(active, room, runtime.content_pin()))
            .then_some(true)
    }

    /// Current custody selects the presentation frame independently of the
    /// origin Channel's offer. A named House retains its existing positive-z wire.
    pub(super) async fn current_actor_native_map_cue_frame(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<bool> {
        use sha2::Digest as _;
        let active = self.active_generation?;
        let room = self.effective_rooms().gate_wire_position()?;
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(session)
                .await
                .ok()?;
        if current.session_state() != GameSessionState::Active {
            return None;
        }
        let runtime = self.runtime.lock().await;
        let pin = runtime.content_pin();
        if runtime.owner_fence().is_err()
            || !source_available(active, room, pin)
            || self.wire_position_frame(&runtime).is_none()
            || self.effective_movement_cells().scope().world_id != pin.world_id()
            || self.effective_movement_cells().scope().generation_digest
                != pin.server_artifact_digest()
            || <[u8; 32]>::from(sha2::Sha256::digest(
                self.effective_movement_cells()
                    .scope()
                    .map_revision
                    .as_str()
                    .as_bytes(),
            )) != pin.map_revision_digest()
        {
            return None;
        }
        let lease = current.current_character_lease();
        if let Ok(house) = runtime.house_actor_state(actor, session) {
            return (house.scope == current.current_runtime_scope()
                && house.generation == current.current_scope_generation()
                && house.lease == lease
                && runtime
                    .house_player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none()))
            .then_some(false);
        }
        (runtime.binding().runtime_scope() == current.current_runtime_scope()
            && runtime.binding().scope_generation() == current.current_scope_generation()
            && runtime.bound_attacker_lease(actor, session).ok() == Some(lease)
            && runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none()))
        .then_some(true)
    }
}

#[cfg(test)]
mod boot_tests {
    #[test]
    fn srx3_qualified_native_cue_boot_and_resume_preserve_exact_selection() -> Result<(), String> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                use crate::gameplay_transport::{
                    capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES as BASE,
                    world_required_capabilities::{
                        BoundRequiredCapabilities, with_native_map_presentation_offer,
                    },
                };
                let scene = crate::gameplay_transport::wire_position::test_scene::source_scene()?;
                let active = scene
                    .controller
                    .active()
                    .ok_or("actual native activation")?;
                let pin = scene.runtime.content_pin();
                let id = |tag: u8| {
                    let mut bytes = [tag; 16];
                    bytes[6] = 0x70;
                    bytes[8] = 0x80;
                    bytes
                };
                let boot = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
                    pin.world_id(),
                    crate::foundation::ChannelId::decode(&id(0x71)).map_err(|e| format!("{e:?}"))?,
                    crate::foundation::NodeId::decode(&id(0x72)).map_err(|e| format!("{e:?}"))?,
                    1, 1, 1, "runtime-scope-assignment:1", 8, pin.clone(),
                ).map_err(|e| format!("{e:?}"))?;
                boot.owner_fence().map_err(|e| format!("{e:?}"))?;
                let offered = with_native_map_presentation_offer(
                    BASE, Some(active), Some(&scene.room), boot.content_pin());
                assert!(
                    offered.iter().any(|offer| offer.id == 23),
                    "BOOT must bind the same qualified presentation offer as fresh Session"
                );
                assert!(
                    offered.iter().any(|offer| offer.id == 385 && offer.requires == [23, 30]),
                    "qualified native BOOT must retain explicit-basis support through later scope handoff"
                );
                let bound = BoundRequiredCapabilities::for_channel_boot(
                    &[11, 18, 30],
                    boot.binding().runtime_scope(),
                    1,
                    "source-cue-boot",
                    offered,
                )
                .map_err(|e| format!("{e:?}"))?;
                let supported = offered.iter().map(|offer| offer.id).collect::<Vec<_>>();
                let first = bound
                    .authenticate_support(&supported)
                    .map_err(|e| format!("{e:?}"))?;
                assert!(first.selected().contains(23));
                assert!(first.selected().contains(385), "both required dependencies select the explicit-basis capability");
                assert!(matches!(scene.runtime.binding().runtime_scope(),
                    crate::foundation::RuntimeScopeRefV1::Instance { .. }),
                    "wire fixture remains the actual destination Instance");
                assert_eq!(with_native_map_presentation_offer(
                    BASE, Some(active), Some(&scene.room), scene.runtime.content_pin()), offered,
                    "qualified destination supports the SAME immutable BOOT selection");
                let restored = bound
                    .authenticate_resume_support(&supported, first.selected())
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(
                    restored.selected(),
                    first.selected(),
                    "House/Instance resume keeps immutable acknowledged selection"
                );
                assert_eq!(
                    with_native_map_presentation_offer(BASE, None, Some(&scene.room), pin),
                    BASE,
                    "no active Native proof cannot add23"
                );
                assert_eq!(
                    with_native_map_presentation_offer(BASE, Some(active), None, pin),
                    BASE,
                    "no actual qualified Bundle cannot add23"
                );
                let wrong = crate::foundation::ChannelContentPin::from_activation(
                    pin.world_id(),
                    pin.activation_sequence(),
                    pin.server_artifact_digest(),
                    pin.client_artifact_digest(),
                    [99; 32],
                    pin.map_revision_digest(),
                    (scene.physical.x, scene.physical.y, scene.physical.floor),
                );
                assert_eq!(
                    with_native_map_presentation_offer(
                        BASE,
                        Some(active),
                        Some(&scene.room),
                        &wrong
                    ),
                    BASE,
                    "stale frame must not reuse a cached qualified offer"
                );
                Ok(())
            })
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| "native BOOT cue test thread panicked")?
    }
}
