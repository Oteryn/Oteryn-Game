use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    match super::super::super::character_info::decode(command.payload) {
        Ok(operation) if admitted.controller.is_some() => {
            Dispatch::CharacterInfo(authority.character_info(*admitted, operation).await)
        }
        _ => Dispatch::CharacterInfo(
            super::super::super::character_info::CharacterInfoReply::Rejected,
        ),
    }
}
