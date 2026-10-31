//! STORE_V1 catalogue page 183 and purchase 185; connection dispatch enforces capability 154.
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    let (status, payload) = authority
        .store_command(
            *admitted,
            command.command_id,
            command.command_type,
            command.payload.to_vec(),
        )
        .await;
    Dispatch::Reply(status, payload)
}
