//! Imbuement command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &Option<SessionItemView>,
) -> Option<Dispatch> {
    Some({
        match authority
            .imbuement_command(
                *admitted,
                command.command_id,
                command.command_type,
                command.payload,
                item_view.as_ref(),
            )
            .await
        {
            transport::imbuement::ImbuementReply::Payload(bytes) => {
                Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes)
            }
            transport::imbuement::ImbuementReply::Rejected => Dispatch::Unregistered,
            transport::imbuement::ImbuementReply::Unavailable => {
                return None;
            }
        }
    })
}
