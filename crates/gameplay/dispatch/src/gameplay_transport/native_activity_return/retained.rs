//! A saved physical return already sealed its source kills and extracted its state.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn has_bound_retained_activity_return(
        &self,
        admitted: AdmittedSession,
        command: Option<u64>,
        reason: ActivityReturnReason,
    ) -> Result<bool, ()> {
        let fence = admitted.item_fence.ok_or(())?;
        let key = ActivityReturnAttemptKey {
            session: admitted.game_session_id,
            connection_generation: admitted.continuity.connection_generation,
            command,
            server_reason: if command.is_some() {
                0
            } else {
                match reason {
                    ActivityReturnReason::Timeout => 2,
                    ActivityReturnReason::Defeated => 3,
                    ActivityReturnReason::Death { .. } => 5,
                    ActivityReturnReason::Logout => 6,
                    ActivityReturnReason::IdleLogout => 7,
                    ActivityReturnReason::Abandoned { .. } => 8,
                    ActivityReturnReason::CapabilityMismatch { .. } => 9,
                    _ => return Err(()),
                }
            },
            death_occurrence: match reason {
                ActivityReturnReason::Death { occurrence } => Some(occurrence),
                _ => None,
            },
        };
        let pending = self
            .activity_runtimes
            .returns
            .lock()
            .map_err(|_| ())?
            .get(&key)
            .cloned();
        let Some(pending) = pending else {
            return Ok(false);
        };
        let held = pending.lock().await;
        if held.trigger_reason != reason
            || held.admitted.controller != admitted.controller
            || held.admitted.item_fence != Some(fence)
            || held.admitted.runtime_actor != admitted.runtime_actor
            || held.request.source != fence
            || held.request.command_id != command
            || held.source.scope != fence.runtime_scope
            || held.source.generation != fence.scope_ownership_generation.get()
        {
            return Err(());
        }
        Ok(held.receipt.is_some()
            || held.outcome.is_none()
            || (command.is_some()
                && matches!(&held.outcome, Some(NativeActivityOutcome::Refused(_)))))
    }
}
