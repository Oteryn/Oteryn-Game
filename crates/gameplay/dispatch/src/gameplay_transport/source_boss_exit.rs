//! Crystal201189 boss_lever.lua:337-338 / teleport.lua:5-27: Ahau's
//! Player-only STEP exit is a shared Instance return, never a Channel teleport.
use super::ComposedFreshAdmission;
use super::connection::AdmittedSession;
use super::native_activity::NativeActivityOutcome;
use crate::content::bosses::activity_template::QualifiedBossExitTeleporter;
use crate::durability::activity_handoff::ActivityNativeStepExit;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeScopeRefV1};
use oteryn_gameplay_entities::carrier::native_translation_events::CommittedMovementOccurrence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceBossExitProof {
    native_step: ActivityNativeStepExit,
    actor: ExactActorRef,
    session: GameSessionId,
    exit: QualifiedBossExitTeleporter,
}
impl SourceBossExitProof {
    pub(super) fn matches(&self, runtime: &ChannelRuntimeV1, admitted: AdmittedSession) -> bool {
        let Some(fence) = admitted.item_fence else {
            return false;
        };
        runtime.owner_fence().is_ok()
            && matches!(
                runtime.binding().runtime_scope(),
                RuntimeScopeRefV1::Instance { .. }
            )
            && admitted.runtime_actor == Some(self.actor)
            && admitted.game_session_id == self.session
            && fence.runtime_scope == runtime.binding().runtime_scope()
            && fence.scope_ownership_generation == runtime.binding().scope_generation()
            && fence.game_session_id == self.session
            && fence.connection_generation.get() == admitted.continuity.connection_generation
            && runtime
                .bound_attacker_lease(self.actor, self.session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
            && runtime
                .player_control_facts(self.actor, self.session)
                .is_ok()
            && ActivityNativeStepExit::from_current_owner(
                runtime,
                self.actor,
                self.session,
                self.movement(),
            )
            .as_ref()
                == Some(&self.native_step)
            && runtime.committed_movement_occurrence(self.actor) == Ok(Some(self.movement()))
            && runtime.read_actor_position(self.actor) == Ok(self.movement().after())
            && {
                let at = self.movement().after().position();
                let trigger = self.exit.trigger();
                at.x == trigger.x && at.y == trigger.y && i32::from(at.floor) == trigger.z
            }
    }
    pub(super) fn movement(&self) -> CommittedMovementOccurrence {
        self.native_step.movement()
    }
    pub(super) fn native_step(&self) -> ActivityNativeStepExit {
        self.native_step.clone()
    }
    pub(super) fn destination(&self) -> crate::content::LogicalCell {
        self.exit.destination()
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Flush the existing position owner's immutable receipt only after the
    /// caller finished its pace bookkeeping and released its movement guards.
    pub(super) async fn source_boss_exit_after_movement(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        moved_to: Option<oteryn_protocol_oteryn::world_spatial::WorldSpatialObservation>,
    ) {
        if !matches!(actor.runtime_scope(), RuntimeScopeRefV1::Instance { .. })
            || moved_to.is_none()
        {
            return;
        }
        let movement = {
            let runtime = self.runtime.lock().await;
            let Ok(Some(movement)) = runtime.committed_movement_occurrence(actor) else {
                return;
            };
            if runtime.read_actor_position(actor) != Ok(movement.after())
                || self.wire_observation(&runtime, movement.after().position()) != moved_to
            {
                return;
            }
            movement
        };
        self.source_boss_exit_movement(actor, session, movement)
            .await;
    }

    /// Observe physical Player movement after all owned movement guards are released.
    pub(super) async fn source_boss_exit_movement(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        movement: CommittedMovementOccurrence,
    ) {
        let Some(admitted) = self.activity_runtimes.connection(session) else {
            return;
        };
        let Some(fence) = admitted.item_fence else {
            return;
        };
        let Some(registered) = self
            .activity_runtimes
            .get(fence.runtime_scope, fence.scope_ownership_generation.get())
        else {
            return;
        };
        let Some(key) = registered.template.as_ref() else {
            return;
        };
        let Some(template) = self.activity_runtimes.template(&key.key, &key.revision) else {
            return;
        };
        let Some(exit) = template.exit_teleporter() else {
            return;
        };
        let proof = {
            let runtime = self.runtime.lock().await;
            let Some(native_step) =
                ActivityNativeStepExit::from_current_owner(&runtime, actor, session, movement)
            else {
                return;
            };
            SourceBossExitProof {
                native_step,
                actor,
                session,
                exit,
            }
        };
        {
            let runtime = self.runtime.lock().await;
            let pin = runtime.content_pin();
            if !proof.matches(&runtime, admitted)
                || template.room().frame_binding().digest() != pin.frame_binding_digest()
                || template.room().map_revision_digest() != pin.map_revision_digest()
                || template.room().compiled().server_digest() != pin.server_artifact_digest()
                || template.room().compiled().client_digest() != pin.client_artifact_digest()
            {
                return;
            }
        }
        let retry = super::AdmissionRetryBudget::for_command(self.root);
        let returned = retry
            .retry_unavailable(async || {
                self.return_source_boss_exit(admitted, proof.clone())
                    .await
                    .map_err(|()| super::connection::AdmissionRefusal::Unavailable)
            })
            .await;
        match returned {
            Ok(NativeActivityOutcome::Committed(commit)) => {
                if self.publish_activity_commit(&commit).is_err() {
                    super::operator_event("source_boss_exit_projection_retry");
                }
            }
            Ok(NativeActivityOutcome::Refused(_)) | Err(_) => {
                super::operator_event("source_boss_exit_return_deferred")
            }
        }
    }
    pub(super) async fn emit_source_boss_exit_effect(
        &self,
        admitted: AdmittedSession,
    ) -> Result<u64, ()> {
        let actor = admitted.runtime_actor.ok_or(())?;
        let mut runtime = self.runtime.lock().await;
        runtime.owner_fence().map_err(|_| ())?;
        if !runtime
            .player_control_facts(actor, admitted.game_session_id)
            .is_ok()
        {
            return Err(());
        }
        let at = super::bundle_map_position(&mut runtime, self.movement_cells(), actor).ok_or(())?;
        let cursor = at
            .map
            .facts
            .movement_effects_since(0)
            .last()
            .map_or(0, |row| row.0);
        let pos = crate::map::overlay::TilePos {
            x: at.at.x.try_into().map_err(|_| ())?,
            y: at.at.y.try_into().map_err(|_| ())?,
            floor: at.at.floor.try_into().map_err(|_| ())?,
        };
        if !at.map.facts.emit_movement_effect(pos, 11) {
            return Err(());
        }
        Ok(cursor)
    }
}
