//! A real logout command first returns a real Instance participant through the
//! retained handoff owner, then uses the existing origin terminal release.
use super::native_activity::NativeActivityOutcome;
use super::{
    ComposedFreshAdmission, connection::AdmittedSession, logout::LogoutReply, operator_event,
};
use crate::durability::activity_handoff::ActivityReturnReason;
use crate::foundation::RuntimeScopeRefV1;
use oteryn_protocol_oteryn::logout::LogoutResult;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn logout_activity_command(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
    ) -> LogoutReply {
        if command_id == 0 {
            return LogoutReply::Result(LogoutResult::Busy);
        }
        self.logout_activity_intent(admitted, Some(command_id))
            .await
    }

    pub(super) async fn logout_activity_idle(&self, admitted: AdmittedSession) -> LogoutReply {
        use super::connection::FreshAdmissionAuthority;
        let now = self.gameplay_idle_now();
        let policy = self.gameplay_idle_policy();
        if admitted.continuity.gameplay_idle.elapsed(now)
            < std::time::Duration::from_millis(policy.removal_after_ms)
        {
            return LogoutReply::Result(LogoutResult::Busy);
        }
        self.logout_activity_intent(admitted, None).await
    }

    /// Account closure has its own durable authority and no inactivity deadline.
    /// Activity departure and the exact actor/session release checks still apply.
    pub(super) async fn logout_activity_closure(&self, admitted: AdmittedSession) -> LogoutReply {
        self.logout_activity_intent(admitted, None).await
    }

    async fn logout_activity_intent(
        &self,
        admitted: AdmittedSession,
        command: Option<u64>,
    ) -> LogoutReply {
        let admitted = self.activity_runtimes.current_connection(admitted);
        let scoped = match self.resolve_runtime_view(admitted).await {
            Ok(scoped) => scoped,
            Err(()) => {
                operator_event("event=logout_unknown stage=activity_view kind=authority");
                return LogoutReply::Unknown;
            }
        };
        let owner = scoped.as_ref().unwrap_or(self);
        if admitted
            .item_fence
            .is_none_or(|fence| !matches!(fence.runtime_scope, RuntimeScopeRefV1::Instance { .. }))
        {
            return owner.logout_session(admitted).await;
        }
        // NOLOGOUT, paid work and PvP/PvE fight gates keep their existing policy.
        // The native return rechecks the combat hold under its reservation locks.
        if let Err(reply) = owner.logout_preflight(admitted).await {
            return reply;
        }
        let returned = match command {
            Some(command_id) => {
                owner
                    .return_native_activity(admitted, command_id, ActivityReturnReason::Logout)
                    .await
            }
            None => {
                owner
                    .return_due_activity(admitted, ActivityReturnReason::IdleLogout)
                    .await
            }
        };
        let commit = match returned {
            Ok(NativeActivityOutcome::Committed(commit)) => commit,
            Ok(NativeActivityOutcome::Refused(_)) => {
                return LogoutReply::Result(LogoutResult::Busy);
            }
            Err(()) => {
                operator_event("event=logout_unknown stage=activity_return kind=unknown");
                return LogoutReply::Unknown;
            }
        };
        if self.publish_activity_commit(&commit).is_err() {
            operator_event("event=logout_unknown stage=activity_publish kind=authority");
            return LogoutReply::Unknown;
        }
        let next = self.activity_runtimes.current_connection(admitted);
        if next.game_session_id == admitted.game_session_id {
            operator_event("event=logout_unknown stage=activity_successor kind=authority");
            return LogoutReply::Unknown;
        }
        let destination = match self.resolve_runtime_view(next).await {
            Ok(destination) => destination,
            Err(()) => {
                operator_event("event=logout_unknown stage=activity_destination kind=authority");
                return LogoutReply::Unknown;
            }
        };
        destination
            .as_ref()
            .unwrap_or(self)
            .logout_session(next)
            .await
    }
}
