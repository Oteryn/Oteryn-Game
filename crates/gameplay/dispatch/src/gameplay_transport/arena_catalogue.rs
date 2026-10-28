//! Read-only Svargrond challenge catalogue on the admitted session owner.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::arena as wire;

macro_rules! authority_methods {
    () => {
        fn arena_catalogue_command(
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
        async fn arena_catalogue_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            payload: &[u8],
        ) -> Option<Vec<u8>> {
            self.arena_catalogue_wire(admitted, id, payload).await
        }
    };
}
pub(super) use composed_methods;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn arena_catalogue_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
            return None;
        };
        if id == 0
            || wire::decode_catalogue_request(payload).is_err()
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_ARENA_V1)
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return None;
        }
        let runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        if admitted.world_id != binding.world_id()
            || admitted.channel_id != binding.channel_id()
            || fence.scope_ownership_generation != binding.scope_generation()
            || runtime.owner_fence().is_err()
            || !runtime
                .player_control_facts(actor, admitted.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return None;
        }
        wire::encode_catalogue().ok()
    }
}
