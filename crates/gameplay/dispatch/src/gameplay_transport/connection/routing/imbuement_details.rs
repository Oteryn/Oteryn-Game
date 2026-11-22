use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{ClientCommandView, imbuement, imbuement_details as wire};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &Option<SessionItemView>,
) -> Option<Dispatch> {
    if command.command_type == wire::COMMAND_TYPE_IMBUEMENT_PROTECTION {
        let selected = admitted.continuity.selected_capabilities;
        let payload = if selected.contains(wire::CAPABILITY_IMBUEMENT_AFFORDABILITY_V1)
            && selected.contains(wire::CAPABILITY_IMBUEMENT_DETAILS_V1)
            && selected.contains(imbuement::CAPABILITY_IMBUE_V1)
            && imbuement::decode_imbuement_request(command.payload).is_ok_and(|request| {
                matches!(request.action, imbuement::ImbuementAction::Apply { .. })
            })
        {
            wire::IMBUEMENT_PROTECTION_REFUSAL.as_bytes().to_vec()
        } else {
            Vec::new()
        };
        return Some(Dispatch::Reply(CommandStatus::Rejected, payload));
    }
    match transport::imbuement_details::query(authority, admitted, command, item_view.as_ref())
        .await
    {
        transport::imbuement::ImbuementReply::Payload(bytes) => {
            Some(Dispatch::Reply(CommandStatus::Accepted, bytes))
        }
        transport::imbuement::ImbuementReply::Rejected => {
            Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()))
        }
        transport::imbuement::ImbuementReply::Unavailable => None,
    }
}
