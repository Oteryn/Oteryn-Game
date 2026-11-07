//! Party command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    party_view: &mut transport::party_command::SessionPartyView,
) -> Dispatch {
    match authority
        .party_command(
            *admitted,
            command.command_id,
            command.command_type,
            command.payload,
            party_view,
        )
        .await
    {
        Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
        None => Dispatch::Unregistered,
    }
}
