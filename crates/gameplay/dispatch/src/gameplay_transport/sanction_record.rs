//! N009: the admitted Character's own Platform sanction record, a fenced read of Game's
//! projection (`sanction-record-v1.md`). Platform remains the only sanction authority.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::foundation::CommandId;
use oteryn_protocol_oteryn::sanction_record as wire;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn sanction_record_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        wire::decode_query(payload)?;
        let fence = admitted.item_fence?;
        CommandId::new(id).ok()?;
        {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_SANCTION_RECORD_V1)
                || fence.game_session_id != admitted.game_session_id
                || fence.connection_generation.get() != admitted.continuity.connection_generation
                || admitted.world_id != binding.world_id()
                || Some(admitted.channel_id) != binding.channel_id()
                || fence.runtime_scope != binding.runtime_scope()
                || fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
            {
                return None;
            }
        }
        self.root
            .read_own_sanction_record(self.character, self.holder, fence)
            .await
            .ok()
            .flatten()
    }
}
macro_rules! authority_methods {
    () => {
        fn sanction_record_query(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn sanction_record_query(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            payload: &[u8],
        ) -> Option<Vec<u8>> {
            self.sanction_record_wire(admitted, id, payload).await
        }
    };
}
pub(super) use composed_methods;
