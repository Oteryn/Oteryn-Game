//! Chat command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{AdmittedSession, ExactActorRef, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    if command.command_type == oteryn_protocol_oteryn::chat::console::COMMAND {
        use oteryn_protocol_oteryn::chat::console;
        if !admitted.continuity.selected_capabilities.contains(7) {
            return Some(Dispatch::Unregistered);
        }
        let Some(names) = console::decode_names(command.payload) else {
            return Some(Dispatch::Unregistered);
        };
        return Some(
            match authority
                .console_levels(*admitted, names)
                .await
                .and_then(|levels| console::encode_levels(&levels))
            {
                Some(payload) => {
                    Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload)
                }
                None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
            },
        );
    }
    // CHAT-WIRE-1: spam control is the speaker's limiter inside the authority; a
    // malformed intent is REJECTED with an empty payload like any unregistered command.
    match transport::guild_chat::wire::dispatch(
        authority,
        *admitted,
        actor,
        command.command_id,
        command.payload,
    )
    .await
    {
        Ok(Some((result, spell))) => Some(Dispatch::Chat(result, spell)),
        Ok(None) => None,
        Err(()) => Some(Dispatch::Unregistered),
    }
}
