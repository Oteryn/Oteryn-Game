use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::gameplay_transport::guild_intent::GuildReply;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    // Lever input reaches Quest prerequisites but the unqualified shared owner still refuses.
    // Client terminal commands cannot manufacture death, timeout or admission authority.
    if !matches!(
        command.command_type,
        oteryn_protocol_oteryn::boss_encounter::COMMAND_TYPE_BOSS_COOLDOWNS
            | oteryn_protocol_oteryn::boss_encounter::COMMAND_TYPE_BOSS_REWARD_OPEN
            | oteryn_protocol_oteryn::boss_encounter::COMMAND_TYPE_BOSS_LEVER
    ) {
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
