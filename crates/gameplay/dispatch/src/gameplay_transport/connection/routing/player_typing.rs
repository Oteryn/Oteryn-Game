use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::player_typing::decode_intent;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let Ok(typing) = decode_intent(command.payload) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    Some(Dispatch::Reply(
        if authority.player_typing(*admitted, typing).await {
            CommandStatus::Accepted
        } else {
            CommandStatus::Rejected
        },
        Vec::new(),
    ))
}
