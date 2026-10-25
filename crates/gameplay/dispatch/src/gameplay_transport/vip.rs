//! VIP-1 server command composition; no presence/UI surface is installed here.
use super::ComposedFreshAdmission;
use super::connection::{AdmittedSession, FreshAdmissionAuthority};
use crate::domain::premium::PremiumStatus;
use crate::durability::vip::VipPrepared;
use oteryn_protocol_oteryn::vip::{self as wire, Operation};

macro_rules! authority_methods {
    () => {
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
        VipPrepared::Fresh(write) if growth => owner
            .with_premium_gate(write.account, move |status, tx| {
                Box::pin(async move { write.commit(status, tx).await })
            })
            .await
            .ok()?
            .ok()?,
        VipPrepared::Fresh(write) => write.commit(PremiumStatus::NotCurrent, None).await.ok()?,
    };
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
