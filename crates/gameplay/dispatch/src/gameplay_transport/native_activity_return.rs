//! Live return half of ActivityHandoffOwner: reserve, retain, commit and rebind.
//! The retained source room supplies its exit; no client chooses a Channel/cell.
use super::activity_runtime::{RegisteredRuntime, ScopeKey};
use super::actor_spell::PreparedActivitySpellStates;
use super::attack::PreparedActivityAttackStates;
use super::connection::{
    AdmittedSession, ConnectionIdentifiers, ControllerBinding, FirstEntryOutcome,
};
use super::kill_reward::{DrainOutcome, KillReleaseMark, ReleaseEnd};
use super::native_activity::{
    NativeActivityCommit, NativeActivityOutcome, activity_continuity, finish_activity_kills,
    seal_activity_kills,
};
use super::{AdmissionRetryBudget, ComposedFreshAdmission, SecureIdentifiers};
use crate::durability::activity_handoff::{
    ActivityHandoffOwner, ActivityRefusal, ActivityReturnOutcome, ActivityReturnPlacement,
    ActivityReturnReason, ActivityReturnReceipt, ActivityReturnRequest,
};
use crate::durability::character_last_position::LastVitals;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ActivityTransferMember, ActivityTransferReservation, ActivityTransferredActor, CharacterLease,
    ControlLossMark, GameSessionId, GameSessionState, MovementLocalPosition, RuntimeScopeRefV1,
};
use std::sync::Arc;

/// Client commands and autonomous lifecycle occurrences occupy separate keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ActivityReturnAttemptKey {
    session: GameSessionId,
    connection_generation: u64,
    command: Option<u64>,
    server_reason: u8,
    death_occurrence: Option<[u8; 16]>,
    native_step_occurrence: Option<[u8; 32]>,
}

pub(super) struct NativeActivityReturnAttempt<'a> {
    source: RegisteredRuntime<'a>,
    source_exit: Option<super::source_boss_exit::SourceBossExitProof>,
    exit_effect_start: Option<u64>,
    destination: RegisteredRuntime<'a>,
    admitted: AdmittedSession,
    // The original autonomous trigger keeps its exact retry identity even when
    // a proven native death selects the temple return instead of a room exit.
    trigger_reason: ActivityReturnReason,
    placement: ActivityReturnPlacement,
    request: ActivityReturnRequest,
    physical: ActivityTransferReservation,
    spells: PreparedActivitySpellStates,
    attacks: PreparedActivityAttackStates,
    kill_marks: Vec<KillReleaseMark>,
    receipt: Option<ActivityReturnReceipt>,
    moved: Option<ActivityTransferredActor>,
    states_installed: bool,
    ancillary_installed: bool,
    death_completed: bool,
    outcome: Option<NativeActivityOutcome>,
}

impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    pub(super) async fn prune_refused_activity_returns(
        &self,
        policy: super::activity_runtime::ActivityRefusalPrune,
    ) -> Result<(), ()> {
        let entries: Vec<_> = self
            .activity_runtimes
            .returns
            .lock()
            .map_err(|_| ())?
            .iter()
            .filter(|(key, _)| policy.includes(key.session, key.connection_generation, key.command))
            .map(|(key, pending)| (*key, Arc::clone(pending)))
            .collect();
        for (key, pending) in entries {
            let held = pending.lock().await;
            if held.receipt.is_some()
                || !matches!(held.outcome, Some(NativeActivityOutcome::Refused(_)))
            {
                continue;
            }
            let mut entries = self.activity_runtimes.returns.lock().map_err(|_| ())?;
            if entries
                .get(&key)
                .is_some_and(|current| Arc::ptr_eq(current, &pending))
            {
                entries.remove(&key);
            }
        }
        Ok(())
    }

    /// Only fully completed returns may release their retained source owner.
    /// Unknown SQL/native/respawn outcomes block retirement and retain tokens.
    pub(super) async fn prune_completed_activity_returns(
        &self,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> Result<(), ()> {
        let attempts: Vec<_> = self
            .activity_runtimes
            .returns
            .lock()
            .map_err(|_| ())?
            .iter()
            .map(|(key, attempt)| (*key, Arc::clone(attempt)))
            .collect();
        for (key, pending) in attempts {
            let held = pending.lock().await;
            if held.source.scope != scope || held.source.generation != generation {
                continue;
            }
            if held.outcome.is_none() {
                return Err(());
            }
            self.activity_runtimes
                .returns
                .lock()
                .map_err(|_| ())?
                .remove(&key);
        }
        Ok(())
    }

    pub(super) async fn return_native_activity(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
        reason: ActivityReturnReason,
    ) -> Result<NativeActivityOutcome, ()> {
        if command_id == 0
            || matches!(
                reason,
                ActivityReturnReason::Death { .. }
                    | ActivityReturnReason::OwnerLost
                    | ActivityReturnReason::IdleLogout
                    | ActivityReturnReason::StaffKickLost { .. }
                    | ActivityReturnReason::StaffKick { .. }
                    | ActivityReturnReason::Abandoned { .. }
                    | ActivityReturnReason::CapabilityMismatch { .. }
            )
        {
            return Ok(NativeActivityOutcome::Refused(
                ActivityRefusal::InvalidInput,
            ));
        }
        self.prepare_native_activity_return(admitted, Some(command_id), reason, None)
            .await
    }

    /// The autonomous owner supplies only a reason; the retained return UUID is
    /// the server occurrence. No CommandId or CommandResult is manufactured.
    pub(super) async fn return_due_activity(
        &self,
        admitted: AdmittedSession,
        reason: ActivityReturnReason,
    ) -> Result<NativeActivityOutcome, ()> {
        if !matches!(
            reason,
            ActivityReturnReason::Timeout
                | ActivityReturnReason::Defeated
                | ActivityReturnReason::SoulpitTerminal
                | ActivityReturnReason::Death { .. }
                | ActivityReturnReason::Logout
                | ActivityReturnReason::IdleLogout
                | ActivityReturnReason::StaffKickLost { .. }
                | ActivityReturnReason::StaffKick { .. }
                | ActivityReturnReason::Abandoned { .. }
                | ActivityReturnReason::CapabilityMismatch { .. }
        ) {
            return Ok(NativeActivityOutcome::Refused(
                ActivityRefusal::InvalidInput,
            ));
        }
        self.prepare_native_activity_return(admitted, None, reason, None)
            .await
    }

    #[cfg(test)]
    pub(super) async fn source_exit_movement_for_test(
        &self,
        session: GameSessionId,
    ) -> Option<
        oteryn_gameplay_entities::carrier::native_translation_events::CommittedMovementOccurrence,
    > {
        let attempts = self
            .activity_runtimes
            .returns
            .lock()
            .ok()?
            .iter()
            .filter(|(key, _)| key.session == session)
            .map(|(_, held)| std::sync::Arc::clone(held))
            .collect::<Vec<_>>();
        for attempt in attempts {
            let held = attempt.lock().await;
            if let Some(proof) = &held.source_exit {
                return Some(proof.movement());
            }
        }
        None
    }

    pub(super) async fn return_source_boss_exit(
        &self,
        admitted: AdmittedSession,
        proof: super::source_boss_exit::SourceBossExitProof,
    ) -> Result<NativeActivityOutcome, ()> {
        self.prepare_native_activity_return(admitted, None, ActivityReturnReason::Exit, Some(proof))
            .await
    }

    /// Resolve retained return tokens before a socket-end handler records loss.
    /// A lost scope acknowledgement cannot make the old Instance session live.
    pub(super) async fn settle_native_return_connection(
        &self,
        admitted: AdmittedSession,
    ) -> Result<AdmittedSession, ()> {
        let admitted = self.activity_runtimes.current_connection(admitted);
        let attempts: Vec<_> = self
            .activity_runtimes
            .returns
            .lock()
            .map_err(|_| ())?
            .values()
            .cloned()
            .collect();
        for pending in attempts {
            let _transition = self.activity_runtimes.transitions.lock().await;
            let held = pending.lock().await;
            let belongs = held.admitted.game_session_id == admitted.game_session_id
                && held.admitted.controller == admitted.controller
                && held.admitted.item_fence == admitted.item_fence;
            drop(held);
            if !belongs {
                continue;
            }
            if let NativeActivityOutcome::Committed(commit) =
                self.finish_native_activity_return(pending).await?
            {
                self.publish_activity_commit(&commit)?;
                return Ok(self.activity_runtimes.current_connection(admitted));
            }
            // A prior definite refusal may belong to this same connection.
            // It cannot hide a later retained attempt whose commit is unknown;
            // HashMap iteration order is not a semantic ordering of commands.
        }
        Ok(admitted)
    }

    pub(super) async fn drain_native_return_attempts(&self) -> Result<(), ()> {
        let mut attempts: Vec<_> = self
            .activity_runtimes
            .returns
            .lock()
            .map_err(|_| ())?
            .iter()
            .map(|(key, pending)| (*key, Arc::clone(pending)))
            .collect();
        attempts.sort_by_key(|(key, _)| {
            (
                *key.session.as_bytes(),
                key.connection_generation,
                key.command,
            )
        });
        let mut complete = true;
        let start = self.background_start("activity-returns", attempts.len());
        let count = attempts.len();
        for index in (start..count).chain(0..start) {
            let pending = Arc::clone(&attempts[index].1);
            let _transition = self.activity_runtimes.transitions.lock().await;
            match self
                .background_unit(
                    "activity-returns",
                    index + 1,
                    self.finish_native_activity_return(pending),
                )
                .await
            {
                Ok(NativeActivityOutcome::Committed(commit)) => {
                    self.publish_activity_commit(&commit)?
                }
                Ok(NativeActivityOutcome::Refused(_)) => {}
                Err(()) => complete = false,
            }
        }
        if complete { Ok(()) } else { Err(()) }
    }
}

include!("native_activity_return/prepare.rs");

include!("native_activity_return/finish.rs");

fn return_receipt_matches(
    request: &ActivityReturnRequest,
    placement: &ActivityReturnPlacement,
    receipt: &ActivityReturnReceipt,
) -> bool {
    receipt.return_id == request.return_id
        && receipt.handoff_id == request.handoff_id
        && receipt.character_id.as_bytes() == request.source.character_id.as_bytes()
        && receipt.source_game_session_id == request.source.game_session_id
        && receipt.destination_game_session_id == request.destination_game_session_id
        && receipt.destination_scope == placement.destination_scope
        && receipt.destination_generation == placement.destination_generation
        && Some(receipt.destination_lease_generation)
            == request.source.character_lease_generation.checked_add(1)
        && receipt.destination_transport == request.destination_transport
        && receipt.exit_cell == placement.exit_position.cell
}
