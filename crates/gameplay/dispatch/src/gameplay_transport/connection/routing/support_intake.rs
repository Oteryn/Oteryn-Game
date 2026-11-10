//! Support intake never disconnects gameplay when optional evidence storage is unavailable.

use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    match authority
        .support_intake_command(
            *admitted,
            command.command_id,
            command.command_type,
            command.payload,
        )
        .await
    {
        transport::support_intake::SupportIntakeOutcome::Payload(bytes) => {
            Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes)
        }
        transport::support_intake::SupportIntakeOutcome::Rejected
        | transport::support_intake::SupportIntakeOutcome::Unavailable => Dispatch::Unregistered,
    }
}
