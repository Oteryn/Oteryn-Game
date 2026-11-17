use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::ClientCommandView;
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    let Some(definition) =
        oteryn_protocol_oteryn::weapon_proficiency_state::decode_request(command.payload)
    else {
        return Dispatch::Reply(CommandStatus::Rejected, Vec::new());
    };
    match authority
        .weapon_proficiency_state_query(*admitted, definition)
        .await
    {
        Some(payload) => Dispatch::Reply(CommandStatus::Accepted, payload),
        None => Dispatch::Reply(CommandStatus::Rejected, Vec::new()),
    }
}
