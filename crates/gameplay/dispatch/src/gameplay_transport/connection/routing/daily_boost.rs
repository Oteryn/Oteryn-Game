use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::gameplay_transport::daily_boost::{DailyBoostReply, is_retryable};
use oteryn_protocol_oteryn::ClientCommandView;

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
            DailyBoostReply::Payload(bytes) => {
                Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes)
            }
            DailyBoostReply::Unavailable(error) if is_retryable(&error) => Dispatch::Reply(
                crate::foundation::CommandStatus::Rejected,
                oteryn_protocol_oteryn::COMMAND_BUSY_REFUSAL.to_vec(),
            ),
            DailyBoostReply::Unavailable(_) => {
                Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new())
            }
            DailyBoostReply::Rejected => Dispatch::Unregistered,
        }
}
