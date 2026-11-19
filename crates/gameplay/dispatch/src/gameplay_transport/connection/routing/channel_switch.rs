//! chsw: in-game Channel switch. An accepted switch closes the transport after its result,
//! exactly as an accepted logout; an unknown release closes without one.
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::foundation::CommandStatus;
use crate::gameplay_transport::channel_switch::ChannelSwitchOutcome;
use crate::gameplay_transport::server_modal::ServerModalOutcome;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    match authority
        .channel_switch_execute(
            *admitted,
            command.command_id,
            command.command_type,
            command.payload,
        )
        .await
    {
        Some(ChannelSwitchOutcome::Status(bytes)) => Dispatch::Reply(CommandStatus::Accepted, bytes),
        Some(ChannelSwitchOutcome::Refused(bytes)) => Dispatch::Reply(CommandStatus::Rejected, bytes),
        // The terminal-release-then-close outcome shared with the bed modal's logout.
        Some(ChannelSwitchOutcome::Switched(bytes)) => {
            Dispatch::ServerModal(ServerModalOutcome::LoggedOut(bytes))
        }
        Some(ChannelSwitchOutcome::Unknown) => Dispatch::ServerModal(ServerModalOutcome::Unavailable),
        None => Dispatch::Reply(CommandStatus::Rejected, Vec::new()),
    }
}
