//! House command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{AdmittedSession, ExactActorRef, FreshAdmissionAuthority, UseCommand};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    use oteryn_protocol_oteryn::house as wire;
    if command.command_type == wire::COMMAND_TYPE_HOUSE_INTENT
        && wire::decode_house_intent(command.payload).is_ok_and(|intent| {
            transport::house_query::source_house_kind(&intent.house)
                == Some(transport::house_query::CatalogueKind::Guildhall)
        })
        && !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_GUILDHALL_V1)
    {
        return Dispatch::House(Some(wire::encode_house_result(wire::HouseDisposition::Rejected, 0)));
    }
    Dispatch::House(
        transport::house_intent::dispatch(
            authority,
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command.command_id,
                item_fence: admitted.item_fence,
            },
            command.payload,
            command.command_type == oteryn_protocol_oteryn::house::COMMAND_TYPE_HOUSE_QUERY,
            admitted.continuity.selected_capabilities.contains(wire::CAPABILITY_GUILDHALL_V1),
        )
        .await,
    )
}
