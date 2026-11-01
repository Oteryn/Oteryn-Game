//! Bounded Team Finder member commands on the admitted command stream.
use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    view: &mut transport::team_finder_members::SessionTeamFinderView,
) -> Option<Dispatch> {
    let (accepted, payload) = authority
        .team_finder_members_command(
            *admitted,
            command.command_id,
            command.command_type,
            command.payload,
            view,
        )
        .await?;
    Some(Dispatch::Reply(
        if accepted {
            CommandStatus::Accepted
        } else {
            CommandStatus::Rejected
        },
        payload,
    ))
}
