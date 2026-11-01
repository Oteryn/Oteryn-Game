//! VIP command composition; optional capability 258 adds World presence.
use super::ComposedFreshAdmission;
use super::connection::{AdmittedSession, FreshAdmissionAuthority};
use crate::domain::premium::PremiumStatus;
use crate::durability::vip::VipPrepared;
use oteryn_protocol_oteryn::vip::{self as wire, Operation};

macro_rules! authority_methods {
    () => {
        fn vip_presence_refresh(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
        fn vip_command(
            &self,
            _admitted: AdmittedSession,
            _command_id: u64,
            _kind: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { oteryn_protocol_oteryn::vip::encode_result(9, 0, 0, 0, "").ok() }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn vip_presence_refresh(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
        ) -> Option<Vec<u8>> {
            self.refresh_vip_presence(admitted).await
        }
        async fn vip_command(
            &self,
            admitted: AdmittedSession,
            command_id: u64,
            kind: u32,
            payload: &[u8],
        ) -> Option<Vec<u8>> {
            crate::gameplay_transport::vip::execute(self, admitted, command_id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn dispatch(
    authority: &impl FreshAdmissionAuthority,
    admitted: AdmittedSession,
    command_id: u64,
    kind: u32,
    payload: &[u8],
) -> Option<Vec<u8>> {
    authority
        .vip_command(admitted, command_id, kind, payload)
        .await
}
pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    kind: u32,
    payload: &[u8],
) -> Option<Vec<u8>> {
    let refused = || wire::encode_result(10, 0, 0, 0, "").ok();
    let Ok(request) = wire::decode_request(kind, payload) else {
        return refused();
    };
    let Some(fence) = admitted.item_fence else {
        return refused();
    };
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_VIP_V1)
        || admitted.runtime_actor.is_none()
        || admitted.world_id != fence.runtime_scope.world_id()
        || fence.runtime_scope
            != crate::foundation::RuntimeScopeRefV1::channel(admitted.world_id, admitted.channel_id)
        || fence.game_session_id != admitted.game_session_id
        || fence.connection_generation.get() != admitted.continuity.connection_generation
    {
        return refused();
    }
    let is_query = request.operation == Operation::Query;
    let presence_selected = admitted.continuity.selected_capabilities.contains(
        oteryn_protocol_oteryn::vip_presence::CAPABILITY_VIP_PRESENCE_V1,
    );
    let growth = matches!(
        request.operation,
        Operation::Add | Operation::GroupAdd | Operation::GroupRename
    );
    let prepared = owner
        .root
        .prepare_vip(owner.character, owner.holder, fence, command, request)
        .await
        .ok()?;
    let result = match prepared {
        VipPrepared::Replayed(result) => result,
        // Immutable replay above remains available while the relay is down. A new write
        // cannot grant visibility until this node can observe the World relay again.
        VipPrepared::Fresh(write) if !is_query && presence_selected
            && !owner.states.chat.relay().is_some_and(super::chat_relay::ChatRelay::is_ready) => {
                write.refuse_unavailable().await.ok()?
            }
        VipPrepared::Fresh(write) if growth => owner
            .with_premium_gate(write.account, move |status, tx| {
                Box::pin(async move { write.commit(status, tx).await })
            })
            .await
            .ok()?
            .ok()?,
        VipPrepared::Fresh(write) => write.commit(PremiumStatus::NotCurrent, None).await.ok()?,
    };
    if !is_query && result.get("code").and_then(serde_json::Value::as_u64) == Some(1) {
        owner.states.chat.vip.invalidate();
    }
    wire::encode_result(
        u8::try_from(result.get("code")?.as_u64()?).ok()?,
        result.get("revision")?.as_u64()?,
        result.get("number")?.as_u64()?,
        u8::try_from(result.get("wait")?.as_u64()?).ok()?,
        &result
            .get("view")
            .map(serde_json::Value::to_string)
            .unwrap_or_default(),
    )
    .ok()
}
