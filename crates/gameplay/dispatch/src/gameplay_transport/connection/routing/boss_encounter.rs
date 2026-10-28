use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::gameplay_transport::guild_intent::GuildReply;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    // Reserved activity commands cannot turn a client boss key into death, timeout or
    // admission authority. Keep them refused until the complete Instance ownership flow.
    if command.command_type != oteryn_protocol_oteryn::boss_encounter::COMMAND_TYPE_BOSS_COOLDOWNS {
        return Some(Dispatch::Reply(
            crate::foundation::CommandStatus::Rejected,
            Vec::new(),
        ));
    }
    Some(
        match authority
            .boss_cooldowns_command(
                *admitted,
                command.command_id,
                command.command_type,
                command.payload,
            )
            .await
        {
            GuildReply::Payload(bytes) => {
                Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes)
            }
            GuildReply::Rejected => Dispatch::Unregistered,
            GuildReply::Unavailable => return None,
        },
    )
}
