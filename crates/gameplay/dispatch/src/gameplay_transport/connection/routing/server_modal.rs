use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::server_modal::{
    Answer, CAPABILITY_SERVER_MODAL_V1, COMMAND_TYPE_HIRELING_STORE_SELECTION,
    COMMAND_TYPE_SERVER_MODAL_ANSWER, COMMAND_TYPE_SERVER_MODAL_QUERY,
};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    let valid = match command.command_type {
        COMMAND_TYPE_SERVER_MODAL_QUERY => {
            oteryn_protocol_oteryn::server_modal::decode_query(command.payload).is_ok()
        }
        COMMAND_TYPE_SERVER_MODAL_ANSWER => Answer::decode(command.payload).is_ok(),
        COMMAND_TYPE_HIRELING_STORE_SELECTION => {
            oteryn_protocol_oteryn::store::decode_purchase_request_with_hireling(command.payload)
                .is_ok()
        }
        _ => false,
    };
    if !valid
        || !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_SERVER_MODAL_V1)
    {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    }
    match authority
        .server_modal_execute(
            *admitted,
            command.command_id,
            command.command_type,
            command.payload,
        )
        .await
    {
        Some(crate::gameplay_transport::server_modal::ServerModalOutcome::Reply(reply)) => {
            Dispatch::Reply(crate::foundation::CommandStatus::Accepted, reply)
        }
        Some(outcome) => Dispatch::ServerModal(outcome),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}

#[cfg(test)]
#[path = "server_modal/tests.rs"]
mod tests;
