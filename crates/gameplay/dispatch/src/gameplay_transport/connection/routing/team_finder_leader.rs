use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::team_finder::{
    COMMAND_TYPE_TEAM_FINDER_LEADER_UPSERT, decode_remove, decode_upsert,
};
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let listing = if command.command_type == COMMAND_TYPE_TEAM_FINDER_LEADER_UPSERT {
        decode_upsert(command.payload).map(Some)
    } else {
        decode_remove(command.payload).map(|()| None)
    };
    let Ok(listing) = listing else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    let accepted = authority
        .team_finder_leader(*admitted, command.command_id, listing)
        .await?;
    Some(Dispatch::Reply(
        if accepted {
            CommandStatus::Accepted
        } else {
            CommandStatus::Rejected
        },
        Vec::new(),
    ))
}
