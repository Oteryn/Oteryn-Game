use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{ClientCommandView, daily_boost::DailyBoostRefusal};

pub(in super::super) fn refuse(
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    reason: DailyBoostRefusal,
) -> Dispatch {
    crate::gameplay_transport::operator_event(&format!(
        "event=daily_boost_refused command_id={} seq={} reason={reason:?}",
        command.command_id,
        admitted.continuity.server_sequence.saturating_add(1),
    ));
    Dispatch::Reply(CommandStatus::Rejected, reason.encode().to_vec())
}

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    match authority
        .daily_boost_command(
            *admitted,
            command.command_id,
            command.command_type,
            command.payload,
        )
        .await
    {
        Ok(bytes) => Dispatch::Reply(CommandStatus::Accepted, bytes),
        Err(reason) => refuse(admitted, command, reason),
    }
}
