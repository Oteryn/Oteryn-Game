//! PARITY-REPORT-1 player reports composed from the admitted actor and current Channel owner.
//! The reporter identity and position are server facts; the client supplies only the report.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::player_report::PlayerReportPosition;
use crate::foundation::{CommandId, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::player_report as wire;

#[derive(Debug)]
pub enum PlayerReportOutcome {
    Rejected,
    Payload(Vec<u8>),
    Unavailable,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn player_report_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> PlayerReportOutcome {
        let Ok(request) = wire::decode_player_report_request(payload) else {
            return PlayerReportOutcome::Rejected;
        };
        let (Some(fence), Some(actor), Ok(_)) =
            (admitted.item_fence, admitted.runtime_actor, CommandId::new(id))
        else {
            return PlayerReportOutcome::Rejected;
        };
        let position = {
            let mut runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if kind != wire::COMMAND_TYPE_PLAYER_REPORT
                || !admitted
                    .continuity
                    .selected_capabilities
                    .contains(wire::CAPABILITY_PLAYER_REPORT_V1)
                || fence.game_session_id != admitted.game_session_id
                || fence.connection_generation.get() != admitted.continuity.connection_generation
                || admitted.world_id != binding.world_id()
                || admitted.channel_id != binding.channel_id()
                || fence.runtime_scope
                    != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
                || fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return PlayerReportOutcome::Rejected;
            }
            let context = runtime.pinned_movement_context();
            let Ok(position) = runtime.borrow_movement_position().read(actor) else {
                return PlayerReportOutcome::Rejected;
            };
            if position.context() != context {
                return PlayerReportOutcome::Rejected;
            }
            let position = position.position();
            PlayerReportPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            }
        };
        match self
            .root
            .submit_player_report(self.character, self.holder, fence, id, &request, position)
            .await
        {
            Ok(Some(reply)) => wire::encode_player_report_reply(&reply)
                .map_or(PlayerReportOutcome::Unavailable, PlayerReportOutcome::Payload),
            Ok(None) => PlayerReportOutcome::Rejected,
            Err(_) => PlayerReportOutcome::Unavailable,
        }
    }
}
macro_rules! authority_methods {
    () => {
        fn player_report_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _kind: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<
            Output = $crate::gameplay_transport::player_report::PlayerReportOutcome,
        > {
            async { $crate::gameplay_transport::player_report::PlayerReportOutcome::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn player_report_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> player_report::PlayerReportOutcome {
            self.player_report_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;
