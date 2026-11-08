use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::analyser::CAPABILITY_ANALYSER_V1;
use oteryn_protocol_oteryn::analyser_catalogue::decode_request;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_ANALYSER_V1)
    {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    }
    let Ok(ids) = decode_request(command.payload) else {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    };
    match authority.analyser_catalogue_query(*admitted, &ids).await {
        Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}

#[cfg(test)]
#[path = "analyser_catalogue/tests.rs"]
mod tests;
