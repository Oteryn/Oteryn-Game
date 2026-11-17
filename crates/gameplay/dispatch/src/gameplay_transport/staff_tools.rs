//! NX-1 signed donor staff tools: transport selection never grants authority.
use super::connection::{AdmittedSession, FreshAdmissionAuthority};
use super::{ComposedFreshAdmission, StaffReply};
use oteryn_protocol_oteryn::{staff, staff_tools as wire};
#[path = "staff_world_tools.rs"]
mod world;
pub(super) use world::WorldTools;

fn trusted_clock() -> Option<(u64, u64)> {
    #[cfg(test)]
    if let Some(now) = super::staff::test_clock::get() {
        return now;
    }
    let time = crate::premium::clock::system_trusted_now()?;
    Some((
        u64::try_from(time.lower_us()).ok()? / 1000,
        u64::try_from(time.upper_us()).ok()? / 1000,
    ))
}
pub(super) async fn dispatch<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: AdmittedSession,
    command: u32,
    payload: &[u8],
) -> StaffReply {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_STAFF_TOOLS_V1)
        || !matches!(
            command,
            wire::COMMAND_TYPE_STAFF_TOOLS_REQUEST | wire::COMMAND_TYPE_STAFF_TOOLS_RECEIPT
        )
    {
        return StaffReply::Rejected;
    }
    tokio::time::timeout(
        crate::durability::DB_PASS_DEADLINE,
        authority.staff_tools_command(
            admitted,
            command == wire::COMMAND_TYPE_STAFF_TOOLS_RECEIPT,
            payload,
        ),
    )
    .await
    .unwrap_or(StaffReply::Unknown)
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn execute_staff_tools(
        &self,
        admitted: AdmittedSession,
        receipt: bool,
        payload: &[u8],
    ) -> StaffReply {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_STAFF_TOOLS_V1)
        {
            return StaffReply::Rejected;
        }
        let (Some(fence), Some(actor), Ok(envelope)) = (
            admitted.item_fence,
            admitted.runtime_actor,
            wire::decode_envelope(payload),
        ) else {
            return StaffReply::Rejected;
        };
        let issuer_revision;
        {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if runtime.owner_fence().is_err()
                || admitted.world_id != binding.world_id()
                || Some(admitted.channel_id) != binding.channel_id()
                || fence.runtime_scope != binding.runtime_scope()
                || fence.scope_ownership_generation != binding.scope_generation()
                || fence.game_session_id != admitted.game_session_id
                || fence.connection_generation.get() != admitted.continuity.connection_generation
                || !runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|f| f.control_loss.is_none())
            {
                return StaffReply::Rejected;
            }
            issuer_revision = match runtime.player_control_revision(actor, admitted.game_session_id)
            {
                Ok(revision) => revision,
                Err(_) => return StaffReply::Rejected,
            };
        }
        let request = if receipt {
            None
        } else {
            serde_json::from_slice::<crate::durability::staff::rules::Request>(&envelope.canonical)
                .ok()
        };
        let principal = envelope.principal.clone();
        let result = match self
            .root
            .submit_staff_tools(
                self.character,
                self.holder,
                fence,
                envelope,
                receipt,
                trusted_clock,
            )
            .await
        {
            Ok(Some(bytes)) => match staff::decode_result(&bytes) {
                Ok(result) => result,
                Err(_) => return StaffReply::Unknown,
            },
            Ok(None) => return StaffReply::Rejected,
            Err(error) => {
                #[cfg(test)]
                eprintln!("NX1 submit_staff_tools failed: {error:?}");
                #[cfg(not(test))]
                let _ = error;
                return StaffReply::Unknown;
            }
        };
        let mut info = String::new();
        if result.code == staff::StaffCode::Ok
            && result.disposition == 1
            && let Some(request) = request
        {
            match self
                .finish_staff_world_action(admitted, &principal, &request, issuer_revision)
                .await
            {
                Ok(text) => info = text,
                Err(()) => {
                    #[cfg(test)]
                    eprintln!(
                        "NX1 finish_staff_world_action failed kind={:?}",
                        request.action
                    );
                    return StaffReply::Unknown;
                }
            }
        }
        match wire::encode_reply(&wire::StaffToolsReply { result, info }) {
            Ok(bytes) => StaffReply::Result(bytes),
            Err(_) => StaffReply::Unknown,
        }
    }
}
macro_rules! authority_methods {
    () => {
        fn staff_tools_command(
            &self,
            _admitted: crate::gameplay_transport::connection::AdmittedSession,
            _receipt: bool,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = crate::gameplay_transport::StaffReply> {
            async { crate::gameplay_transport::StaffReply::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn staff_tools_command(
            &self,
            admitted: crate::gameplay_transport::connection::AdmittedSession,
            receipt: bool,
            payload: &[u8],
        ) -> crate::gameplay_transport::StaffReply {
            self.execute_staff_tools(admitted, receipt, payload).await
        }
    };
}
pub(super) use composed_methods;
