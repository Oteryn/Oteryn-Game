//! World event command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority, UseCommand};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    Some({
        match transport::world_event::dispatch(
            authority,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command.command_id,
                item_fence: admitted.item_fence,
            },
            command.payload,
        )
        .await
        {
            Some(Ok(bytes)) => {
                // FND-02 §9: an older same-major peer keeps its original strict result.
                // The durable receipt remains complete; this session's negotiation is stable
                // across replay and reconnect, so projection cannot widen on a retry.
                let bytes = if admitted
                    .continuity
                    .selected_capabilities
                    .contains(oteryn_protocol_oteryn::world_event::CAPABILITY_SEASONAL_EVENTS_V1)
                {
                    bytes
                } else {
                    let mut state =
                        oteryn_protocol_oteryn::world_event::decode_world_event_state(&bytes)
                            .ok()?;
                    state.seasonal.clear();
                    state.cake = None;
                    oteryn_protocol_oteryn::world_event::encode_world_event_state(&state).ok()?
                };
                Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes)
            }
            Some(Err(())) => {
                Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new())
            }
            None => return None,
        }
    })
}
