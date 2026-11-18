//! Cyclopedia command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{
    ClientCommandView, CommandStatus,
    cyclopedia::{CAPABILITY_CYCLOPEDIA_MAP_V1, COMMAND_TYPE_CYCLOPEDIA_SPEED_QUERY},
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    if command.command_type == COMMAND_TYPE_CYCLOPEDIA_SPEED_QUERY {
        if !command.payload.is_empty()
            || !admitted
                .continuity
                .selected_capabilities
                .contains(CAPABILITY_CYCLOPEDIA_MAP_V1)
        {
            return Some(Dispatch::Unregistered);
        }
        // Crystal 201189f8, data/scripts/discovery_system.lua:528-563,605-615:
        // completing discovery grants milestones without a movement-speed effect.
        return Some(Dispatch::Reply(
            CommandStatus::Rejected,
            b"Sorry, not possible.".to_vec(),
        ));
    }
    Some({
        match authority
            .cyclopedia_command(
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
