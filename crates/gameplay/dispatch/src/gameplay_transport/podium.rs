//! PODIUM-1 uses the existing House carrier custody, never the Channel actor position.
use super::{ComposedFreshAdmission, connection};
use oteryn_protocol_oteryn::podium as wire;

/// Shared registered command handler; the connection retains framing and replay ownership.
pub(super) async fn dispatch<A: connection::FreshAdmissionAuthority>(
    authority: &A,
    admitted: &connection::AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Option<(crate::foundation::CommandStatus, Vec<u8>)> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_PODIUM_V1)
    {
        return Some((crate::foundation::CommandStatus::Rejected, Vec::new()));
    }
    let bytes = authority
        .podium_command(*admitted, command.command_id, command.payload)
        .await?;
    let status = if bytes.get(..2) == Some(&[0, 0]) {
        crate::foundation::CommandStatus::Accepted
    } else {
        crate::foundation::CommandStatus::Rejected
    };
    Some((status, bytes))
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn podium_wire(
        &self,
        admitted: connection::AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        let refuse = |code| wire::encode_result(code, 0, wire::Display::default()).ok();
        let Ok(request) = wire::decode_request(payload) else {
            return refuse(1630);
        };
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_PODIUM_V1)
            || admitted.world_id != self.world_id
            || admitted.channel_id != self.channel_id
        {
            return refuse(1631);
        }
        let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
            return refuse(1631);
        };
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || self
                .states
                .player_trades
                .transitioning(admitted.game_session_id)
            || self.states.newhaven_travel.held(admitted.game_session_id)
        {
            return refuse(1631);
        }
        let _lane = self.spell_lane_permit().await?;
        let runtime = self.runtime.lock().await;
        let Ok(state) = runtime.house_actor_state(actor, admitted.game_session_id) else {
            return refuse(1631);
        };
        if state.house != request.house
            || state.scope != fence.runtime_scope
            || state.generation != fence.scope_ownership_generation
            || state.lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || state.lease.generation() != fence.character_lease_generation
        {
            return refuse(1631);
        }
        let Some(content) = self
            .active_generation
            .and_then(|g| g.native_gameplay())
            .and_then(|n| n.cosmetics())
            .cloned()
        else {
            return refuse(1636);
        };
        if runtime.content_pin().server_artifact_digest() != content.source_digest() {
            return refuse(1636);
        }
        let command = crate::foundation::CommandRef::new(
            admitted.game_session_id,
            crate::foundation::CommandId::new(id).ok()?,
        );
        let result = self
            .root
            .podium_command(
                self.character,
                self.holder,
                fence,
                content,
                crate::durability::podium::PodiumCommand {
                    command,
                    request,
                    position: state.position,
                },
            )
            .await;
        result.ok()
    }
}
macro_rules! authority_methods {
    () => {
        fn podium_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async {
                oteryn_protocol_oteryn::podium::encode_result(
                    1631,
                    0,
                    oteryn_protocol_oteryn::podium::Display::default(),
                )
                .ok()
            }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn podium_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            payload: &[u8],
        ) -> Option<Vec<u8>> {
            self.podium_wire(admitted, id, payload).await
        }
    };
}
pub(super) use composed_methods;
