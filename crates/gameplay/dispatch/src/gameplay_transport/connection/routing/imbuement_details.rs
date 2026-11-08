use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::ClientCommandView;
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &Option<SessionItemView>,
) -> Option<Dispatch> {
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
