use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::gameplay_transport::guild_intent::GuildReply;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    Some(
        match authority
            .boss_difficulty_command(*admitted, command.command_id, command.payload)
            .await
        {
            GuildReply::Payload(payload) => {
                Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload)
            }
            GuildReply::Rejected => {
                Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new())
            }
            GuildReply::Unavailable => return None,
        },
    )
}
