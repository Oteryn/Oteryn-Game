//! House move out command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    Some({
        match authority
            .house_move_out_command(
                *admitted,
                command.command_id,
                command.command_type,
                command.payload,
            )
            .await
        {
            transport::house_move_out::HouseMoveOutReply::Payload(bytes) => {
                Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes)
            }
            transport::house_move_out::HouseMoveOutReply::Rejected => Dispatch::Unregistered,
            transport::house_move_out::HouseMoveOutReply::Unavailable => {
                return None;
            }
        }
    })
}
