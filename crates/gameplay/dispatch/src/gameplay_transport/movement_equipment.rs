//! Movement reads the actual initialized Character equipment owner without a
//! cast command. Detached SQL data is rechecked against the current actor turn.
use super::super::ComposedFreshAdmission;
use super::ChannelSpellStates;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_equipment::{EquipmentSnapshot, MovementEquipmentSnapshot};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, GameSessionState, MovementPositionSnapshot,
};
use crate::spell::cast::PlayerSpellState;

/// One bounded read for one STEP attempt/wake. The constructor is private and
/// requires a genuine SQL owner receipt; this object supplies no session grant.
pub(in crate::gameplay_transport) struct MovementEquipmentRead {
    actor: ExactActorRef,
    session: GameSessionId,
    position: MovementPositionSnapshot,
    before: PlayerSpellState,
    read: MovementEquipmentSnapshot,
    delta: i32,
}
impl MovementEquipmentRead {
    /// Call under runtime -> states locks, before movement admission. A buffered
    /// wake obtains a new read; its predecessor includes the existing buffer.
    pub(in crate::gameplay_transport) fn current_delta(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<i32> {
        let fence = self.read.fence();
        if self.actor != actor
            || self.session != session
            || fence.game_session_id != session
            || fence.runtime_scope != runtime.binding().runtime_scope()
            || fence.scope_ownership_generation != runtime.binding().scope_generation()
            || self.read.equipment().content_digest
                != runtime.content_pin().server_artifact_digest()
            || runtime.read_actor_position(actor).ok()? != self.position
            || self.position.context() != runtime.pinned_movement_context()
            || !(runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
                || runtime.forced_player_step_current(actor, session))
            || states.has_pending_spell_commit(actor, session)
            || states.get(runtime, actor, session)? != &self.before
        {
            return None;
        }
        Some(self.delta)
    }
}

fn equipment_delta(equipment: &EquipmentSnapshot, source: &NativeGameplayState) -> Option<i32> {
    super::super::equipment_move::equipment_effects_owner::speed_term(equipment, source)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn read_movement_equipment(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<MovementEquipmentRead> {
        let source = self.active_generation?.native_gameplay()?;
        let (position, before, digest, scope, generation, forced) = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.has_pending_spell_commit(actor, session)
                || !(runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                    || runtime.forced_player_step_current(actor, session))
            {
                return None;
            }
            let position = runtime.read_actor_position(actor).ok()?;
            if position.context() != runtime.pinned_movement_context()
                || source.source_digest() != runtime.content_pin().server_artifact_digest()
            {
                return None;
            }
            (
                position,
                states.get(&runtime, actor, session)?.clone(),
                runtime.content_pin().server_artifact_digest(),
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation(),
                runtime.forced_player_step_current(actor, session),
            )
        };
        let store = FreshAdmissionStore::from_root(self.root.clone());
        use crate::gameplay_transport::connection::AdmissionRefusal::{Rejected, Unavailable};
        // All three reads share the bounded durability holder. Retry an
        // unavailable holder without turning a missing policy or stale fence
        // into an equipment grant. The physical predecessor is checked again
        // by current_delta before the actual Movement commit.
        let (read, delta) = crate::gameplay_transport::AdmissionRetryBudget::for_command(self.root)
            .retry_unavailable(async || {
                let current = store
                    .current_session_at(session)
                    .await
                    .map_err(|_| Unavailable)?
                    .0;
                if current.session_state() != GameSessionState::Active {
                    return Err(Rejected);
                }
                let fence = CurrentCharacterItemFence {
                    character_id: crate::domain::CharacterId::from_bytes(
                        *current.commit().character_id().as_bytes(),
                    )
                    .map_err(|_| Rejected)?,
                    game_session_id: session,
                    connection_generation: current.current_connection_generation(),
                    character_lease_generation: current.current_character_lease().generation(),
                    runtime_scope: current.current_runtime_scope(),
                    scope_ownership_generation: current.current_scope_generation(),
                };
                if fence.runtime_scope != scope || fence.scope_ownership_generation != generation {
                    return Err(Rejected);
                }
                let read = self
                    .root
                    .read_movement_equipment(self.character, self.holder, &fence, digest)
                    .await
                    .map_err(|error| match error {
                        crate::durability::character_equipment::EquipmentError::Rejected(_) => {
                            Rejected
                        }
                        _ => Unavailable,
                    })?;
                if read.fence() != &fence
                    || read.equipment().character != *fence.character_id.as_bytes()
                    || read.equipment().content_digest != digest
                {
                    return Err(Rejected);
                }
                // The SQL reader already locked the current fence. A separately fresh
                // admission observation also rejects handover while that read awaited.
                let after = store
                    .current_session_at(session)
                    .await
                    .map_err(|_| Unavailable)?
                    .0;
                if after.session_state() != GameSessionState::Active
                    || after.commit().character_id() != current.commit().character_id()
                    || after.current_connection_generation() != fence.connection_generation
                    || after.current_character_lease().generation()
                        != fence.character_lease_generation
                    || after.current_runtime_scope() != fence.runtime_scope
                    || after.current_scope_generation() != fence.scope_ownership_generation
                {
                    return Err(Rejected);
                }
                let delta = equipment_delta(read.equipment(), source).ok_or(Rejected)?;
                Ok((read, delta))
            })
            .await
            .ok()?;
        let current = store.current_session_at(session).await.ok()?.0;
        if current.session_state() != GameSessionState::Active
            && !(forced && current.session_state() == GameSessionState::Reconnectable)
        {
            return None;
        }
        let fence = CurrentCharacterItemFence {
            character_id: crate::domain::CharacterId::from_bytes(
                *current.commit().character_id().as_bytes(),
            )
            .ok()?,
            game_session_id: session,
            connection_generation: current.current_connection_generation(),
            character_lease_generation: current.current_character_lease().generation(),
            runtime_scope: current.current_runtime_scope(),
            scope_ownership_generation: current.current_scope_generation(),
        };
        if fence.runtime_scope != scope || fence.scope_ownership_generation != generation {
            return None;
        }
        let read = self
            .root
            .read_movement_equipment(self.character, self.holder, &fence, digest)
            .await
            .ok()?;
        if read.fence() != &fence
            || read.equipment().character != *fence.character_id.as_bytes()
            || read.equipment().content_digest != digest
        {
            return None;
        }
        // The SQL reader already locked the current fence. A separately fresh
        // admission observation also rejects handover while that read awaited.
        let after = store.current_session_at(session).await.ok()?.0;
        if (after.session_state() != GameSessionState::Active
            && !(forced && after.session_state() == GameSessionState::Reconnectable))
            || after.commit().character_id() != current.commit().character_id()
            || after.current_connection_generation() != fence.connection_generation
            || after.current_character_lease().generation() != fence.character_lease_generation
            || after.current_runtime_scope() != fence.runtime_scope
            || after.current_scope_generation() != fence.scope_ownership_generation
        {
            return None;
        }
        let delta = equipment_delta(read.equipment(), source)?;
        Some(MovementEquipmentRead {
            actor,
            session,
            position,
            before,
            read,
            delta,
        })
    }
}
