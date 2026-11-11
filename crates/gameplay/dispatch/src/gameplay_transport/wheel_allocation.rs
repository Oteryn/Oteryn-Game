//! WHEEL-LIVE-1 client allocation boundary.
use super::{ComposedFreshAdmission, connection::AdmittedSession};

use oteryn_gameplay_progression::atelier::vessels as gem_vessels;
use oteryn_protocol_oteryn::wheel as wire;

#[derive(Debug)]
pub enum WheelAllocationReply {
    Rejected,
    Payload(Vec<u8>),
    Unavailable,
}

/// The outcome of one fenced Wheel write: a rule refusal or a commit is a reply; anything
/// else ends the request without one.
pub(super) enum FencedWheelChange {
    Done { refused: bool },
    Rejected,
    Unavailable,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The admitted Character's current item fence when the actor may use the Wheel now: the
    /// session generation, runtime binding, lease and (for a change) the spell lane and vitals.
    pub(super) async fn wheel_change_gate(
        &self,
        admitted: AdmittedSession,
        change: bool,
    ) -> Option<crate::durability::item_transfer::CurrentCharacterItemFence> {
        let item_fence = admitted.item_fence?;
        let runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        if item_fence.game_session_id != admitted.game_session_id
            || item_fence.connection_generation.get() != admitted.continuity.connection_generation
            || admitted.world_id != binding.world_id()
            || Some(admitted.channel_id) != binding.channel_id()
            || item_fence.runtime_scope != binding.runtime_scope()
            || item_fence.scope_ownership_generation != binding.scope_generation()
            || runtime.owner_fence().is_err()
            || !admitted.runtime_actor.is_some_and(|actor| {
                runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                    && runtime
                        .bound_attacker_lease(actor, admitted.game_session_id)
                        .is_ok_and(|lease| {
                            lease.character_id().as_bytes() == item_fence.character_id.as_bytes()
                                && lease.generation() == item_fence.character_lease_generation
                        })
                    && (!change || runtime.assert_actor_spell_unreserved(actor).is_ok())
            })
        {
            return None;
        }
        let states = self.states.spell_states.lock().await;
        if change
            && admitted.runtime_actor.is_none_or(|actor| {
                states.has_pending_spell_commit(actor, admitted.game_session_id)
                    || states
                        .get(&runtime, actor, admitted.game_session_id)
                        .is_none_or(|state| state.health == 0)
            })
        {
            return None;
        }
        Some(item_fence)
    }

    /// Commits one Wheel change through the durable Wheel writer under the gate's fence.
    pub(super) async fn wheel_fenced_change(
        &self,
        admitted: AdmittedSession,
        item_fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        request: crate::durability::character_wheel::WheelChangeRequest,
    ) -> FencedWheelChange {
        let Some(actor) = admitted.runtime_actor else {
            return FencedWheelChange::Rejected;
        };
        // Oteryn owner decision (2026-11-05), diverging from Crystal
        // player_wheel.cpp:2151: removal/reset is allowed anywhere logout's combat
        // lock does not hold the actor, including a current protection-zone tile.
        let removal_permitted = removal_permitted(self, actor, admitted.game_session_id).await;
        let change = self
            .commit_wheel_client(
                admitted.game_session_id,
                super::atelier::GemBuildCommand::Wheel(request),
                removal_permitted,
                Some(item_fence),
            )
            .await;
        match change {
            Ok(super::atelier::GemBuildOutcome::Wheel(outcome)) => FencedWheelChange::Done {
                refused: matches!(
                    *outcome,
                    crate::durability::character_wheel::WheelCommitOutcome::Refused(_)
                ),
            },
            Err(
                crate::durability::character_progression::CharacterProgressionError::InvalidInput
                | crate::durability::character_progression::CharacterProgressionError::AuthorityRejected
                | crate::durability::character_progression::CharacterProgressionError::ConflictingOccurrence
                | crate::durability::character_progression::CharacterProgressionError::MissingProgressionState
                | crate::durability::character_progression::CharacterProgressionError::Unavailable(
                    crate::durability::DurabilityError::RootUnavailable,
                ),
            ) => FencedWheelChange::Rejected,
            _ => FencedWheelChange::Unavailable,
        }
    }

    pub(super) async fn wheel_allocation_wire(
        &self,
        admitted: AdmittedSession,
        kind: u32,
        payload: &[u8],
    ) -> WheelAllocationReply {
        let Ok(request) = wire::decode_wheel_request(payload) else {
            return WheelAllocationReply::Rejected;
        };
        if kind != wire::COMMAND_TYPE_WHEEL_ALLOCATION
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_WHEEL_ALLOCATION_V1)
        {
            return WheelAllocationReply::Rejected;
        }
        let Some(permit) = self.spell_lane_permit().await else {
            return WheelAllocationReply::Rejected;
        };
        let Some(item_fence) = self.wheel_change_gate(admitted, request.action != 0).await else {
            return WheelAllocationReply::Rejected;
        };
        let session = admitted.game_session_id;
        // A vector that cannot be placed into the ordered slots is a rule refusal, like the
        // Wheel writer's own vessel refusal, and writes nothing.
        let refused = if request.action != 0 {
            let Ok(occurrence) =
                crate::durability::character_wheel::WheelOccurrence::from_bytes(request.occurrence)
            else {
                return WheelAllocationReply::Rejected;
            };
            let Ok(slots) = crate::durability::character_wheel::WheelSlots::new(request.points)
            else {
                return WheelAllocationReply::Rejected;
            };
            // Capability 148 clients still send four cells. Their gems are placed into the
            // ordered Gem Vessel slots; the Wheel writer validates the result.
            let vessels = match request.vessels {
                None => Some(None),
                Some(cells) => {
                    let Ok(sessions) = self.states.atelier_sessions.lock() else {
                        return WheelAllocationReply::Unavailable;
                    };
                    let state = sessions.get(&session).and_then(Option::as_ref);
                    let current = state.map_or(gem_vessels::NO_GEM_VESSELS, gem_vessels::from_state);
                    gem_vessels::from_legacy_cells(&current, &cells, |id| {
                        state?
                            .gems
                            .get(&u128::from_be_bytes(id))
                            .map(|gem| gem.quality)
                    })
                    .map(Some)
                }
            };
            match vessels {
                None => true,
                Some(vessels) => {
                    match self
                        .wheel_fenced_change(
                            admitted,
                            item_fence,
                            crate::durability::character_wheel::WheelChangeRequest {
                                occurrence,
                                expected_wheel_revision: request.expected_revision,
                                slots,
                                vessels,
                            },
                        )
                        .await
                    {
                        FencedWheelChange::Done { refused } => refused,
                        FencedWheelChange::Rejected => return WheelAllocationReply::Rejected,
                        FencedWheelChange::Unavailable => {
                            return WheelAllocationReply::Unavailable;
                        }
                    }
                }
            }
        } else {
            false
        };
        drop(permit);
        let allocation = self
            .states
            .wheel_sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(&session).cloned().flatten());
        let Some(allocation) = allocation else {
            // Admission defers this projection when progression or the root is not ready.
            // A query has no outcome to reconcile, so refuse it without ending the stream.
            return if request.action == 0 {
                WheelAllocationReply::Rejected
            } else {
                WheelAllocationReply::Unavailable
            };
        };
        if let Some(actor) = admitted.runtime_actor {
            self.refresh_wheel_vitals(actor, session).await;
        }
        // Old allocation clients receive the exact original reply shape. A vessel-aware
        // save returns the current projection reloaded by the same durable Wheel writer.
        let vessels = if request.vessels.is_some() {
            let Ok(sessions) = self.states.atelier_sessions.lock() else {
                return WheelAllocationReply::Unavailable;
            };
            let vessels: wire::WheelVessels = match sessions.get(&session) {
                Some(Some(state)) => gem_vessels::legacy_cells(&gem_vessels::from_state(state)),
                _ => [None; wire::WHEEL_VESSELS],
            };
            Some(vessels)
        } else {
            None
        };
        wire::encode_wheel_reply(&wire::WheelAllocationReply {
            refused,
            revision: allocation.wheel_revision,
            points: *allocation.slots.points(),
            vessels,
        })
        .map_or(
            WheelAllocationReply::Unavailable,
            WheelAllocationReply::Payload,
        )
    }
}

macro_rules! authority_methods {
    () => {
        fn wheel_allocation_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _kind: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<
            Output = $crate::gameplay_transport::wheel_allocation::WheelAllocationReply,
        > {
            async { $crate::gameplay_transport::wheel_allocation::WheelAllocationReply::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn wheel_allocation_command(
            &self,
            admitted: connection::AdmittedSession,
            kind: u32,
            payload: &[u8],
        ) -> wheel_allocation::WheelAllocationReply {
            self.wheel_allocation_wire(admitted, kind, payload).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn removal_permitted(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> bool {
    let in_protection_zone = owner
        .logout_zones(actor)
        .await
        .and_then(|zones| zones.protection_zone);
    let fact = owner.combat_lock_fact(actor, session).await;
    super::logout::in_fight_logout_retry(
        fact.pvp == super::combat_lock_fact::PvpLockSource::LiveBlocked,
        fact.pve_in_fight_until.map(|until| until.get()),
        fact.sampled_at().get(),
        in_protection_zone,
    )
    .is_none()
}
