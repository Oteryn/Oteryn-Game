//! Preserve the existing abandoned/mismatched terminal-release owners across a
//! native Activity return. No session is retired while its room participant
//! still lacks a committed return, and no client command is manufactured.
use super::connection::{AdmittedSession, FreshAdmissionAuthority, GraceExpiryResult};
use super::native_activity::NativeActivityOutcome;
use super::{ComposedFreshAdmission, TerminalRelease};
use crate::durability::activity_handoff::ActivityReturnReason;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{GameSessionState, RuntimeScopeRefV1};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn release_activity_terminal(
        &self,
        original: AdmittedSession,
        release: TerminalRelease,
    ) -> GraceExpiryResult {
        let admitted = match self.settle_activity_connection(original).await {
            Ok(admitted) => admitted,
            Err(()) => return GraceExpiryResult::Unknown,
        };
        let scoped = match self.resolve_runtime_view(admitted).await {
            Ok(scoped) => scoped,
            Err(()) => return GraceExpiryResult::Unknown,
        };
        let owner = scoped.as_ref().unwrap_or(self);
        let Some(actor) = admitted.runtime_actor else {
            return GraceExpiryResult::NotApplicable;
        };
        // Preserve the existing release preflight before reserving a transfer.
        // Its native return repeats this gate under the actual combat locks.
        while owner
            .hold_while_in_fight(actor, admitted.game_session_id)
            .await
        {}
        let mut death = None;
        if !owner
            .settle_released_death(actor, admitted.game_session_id, &mut death)
            .await
        {
            return GraceExpiryResult::Unknown;
        }
        // A proven pending death uses the Death return and post-death owner.
        // Its exact successor then receives this existing terminal release.
        let mut next = self.activity_runtimes.current_connection(admitted);
        if next.runtime_actor.is_some_and(|actor| {
            matches!(actor.runtime_scope(), RuntimeScopeRefV1::Instance { .. })
        }) {
            let reason = match release {
                TerminalRelease::Abandoned(transport) => {
                    ActivityReturnReason::Abandoned { transport }
                }
                TerminalRelease::CapabilityMismatch(epoch) => {
                    ActivityReturnReason::CapabilityMismatch { epoch }
                }
                // A client logout needs its actual command id and already has
                // its own accepted adapter. This path cannot fabricate one.
                TerminalRelease::Logout(_) => return GraceExpiryResult::Unknown,
            };
            let commit = match owner.return_due_activity(next, reason).await {
                Ok(NativeActivityOutcome::Committed(commit)) => commit,
                Ok(NativeActivityOutcome::Refused(_)) | Err(()) => {
                    return GraceExpiryResult::Unknown;
                }
            };
            if self.publish_activity_commit(&commit).is_err() {
                return GraceExpiryResult::Unknown;
            }
            next = self.activity_runtimes.current_connection(next);
        }
        if next
            .runtime_actor
            .is_none_or(|actor| !matches!(actor.runtime_scope(), RuntimeScopeRefV1::Channel { .. }))
        {
            return GraceExpiryResult::Unknown;
        }
        let destination = match self.resolve_runtime_view(next).await {
            Ok(destination) => destination,
            Err(()) => return GraceExpiryResult::Unknown,
        };
        let destination = destination.as_ref().unwrap_or(self);
        let translated = match release {
            TerminalRelease::Abandoned(_) => {
                let Some(controller) = next.controller else {
                    return GraceExpiryResult::Unknown;
                };
                // The return reserves a genuine new transport. It will never
                // serve this ended socket, so release that exact candidate.
                TerminalRelease::Abandoned(controller.transport)
            }
            TerminalRelease::CapabilityMismatch(_) => {
                let (current, now) = match FreshAdmissionStore::from_root(self.root.clone())
                    .current_session_at(next.game_session_id)
                    .await
                {
                    Ok(current) => current,
                    Err(_) => return GraceExpiryResult::Unknown,
                };
                if current.session_state() == GameSessionState::Terminal {
                    return destination.reconcile_terminal_session(next).await;
                }
                if super::item_fence_of(current) != next.item_fence
                    || current.session_state() != GameSessionState::Reconnectable
                    || current.current_transport().is_some()
                {
                    return GraceExpiryResult::NotApplicable;
                }
                let Some(epoch) = current.current_control_loss_epoch() else {
                    return GraceExpiryResult::Unknown;
                };
                if current
                    .current_original_grace_deadline()
                    .is_some_and(|deadline| deadline <= now)
                {
                    return destination.expire_session_control_loss(next).await;
                }
                TerminalRelease::CapabilityMismatch(epoch)
            }
            TerminalRelease::Logout(_) => return GraceExpiryResult::Unknown,
        };
        let result = destination
            .release_terminal_in_scope(next, translated)
            .await;
        if result == GraceExpiryResult::Released {
            destination.forget_lost(next.game_session_id, next.continuity.connection_generation);
        }
        result
    }
}
