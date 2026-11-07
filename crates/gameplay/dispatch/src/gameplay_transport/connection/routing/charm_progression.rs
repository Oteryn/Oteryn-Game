//! Bestiary and charm progression command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    Some({
        match authority
            .charm_progression_command(
                *admitted,
                command.command_id,
                command.command_type,
                command.payload,
            )
            .await
        {
            transport::guild_intent::GuildReply::Payload(bytes) => Dispatch::Guild(bytes),
            transport::guild_intent::GuildReply::Rejected => Dispatch::Unregistered,
            transport::guild_intent::GuildReply::Unavailable => {
                return None;
            }
        }
    })
}
