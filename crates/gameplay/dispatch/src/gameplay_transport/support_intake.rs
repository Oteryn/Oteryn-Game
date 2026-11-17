//! Consented support intake from the current admitted session, without identity enrichment.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::foundation::CommandId;
use oteryn_protocol_oteryn::support_intake as wire;
#[derive(Debug)]
pub enum SupportIntakeOutcome {
    Rejected,
    Payload(Vec<u8>),
    Unavailable,
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn support_intake_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> SupportIntakeOutcome {
        if wire::decode_submission(payload).is_err() {
            return SupportIntakeOutcome::Rejected;
        }
        let (Some(fence), Some(actor), Ok(_)) = (
            admitted.item_fence,
            admitted.runtime_actor,
            CommandId::new(id),
        ) else {
            return SupportIntakeOutcome::Rejected;
        };
        {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if !matches!(kind, wire::CRASH | wire::SURVEY)
                || !admitted
                    .continuity
                    .selected_capabilities
                    .contains(wire::CAPABILITY_SUPPORT_INTAKE_V1)
                || fence.game_session_id != admitted.game_session_id
                || fence.connection_generation.get() != admitted.continuity.connection_generation
                || admitted.world_id != binding.world_id()
                || Some(admitted.channel_id) != binding.channel_id()
                || fence.runtime_scope != binding.runtime_scope()
                || fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return SupportIntakeOutcome::Rejected;
            }
        }
        match self
            .root
            .submit_support_evidence(self.character, self.holder, fence, kind, payload)
            .await
        {
            Ok(true) => SupportIntakeOutcome::Payload(vec![1]),
            Ok(false) => SupportIntakeOutcome::Rejected,
            Err(_) => SupportIntakeOutcome::Unavailable,
        }
    }
}
macro_rules! authority_methods {
    () => {
        fn support_intake_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _kind: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<
            Output = $crate::gameplay_transport::support_intake::SupportIntakeOutcome,
        > {
            async { $crate::gameplay_transport::support_intake::SupportIntakeOutcome::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn support_intake_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> support_intake::SupportIntakeOutcome {
            self.support_intake_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;
