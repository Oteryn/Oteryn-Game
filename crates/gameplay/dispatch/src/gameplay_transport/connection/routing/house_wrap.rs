use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, house_wrap as wire};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_HOUSE_FURNITURE_WRAP_V1)
    {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    }
    let Some(request) = decode_request(
        admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_HOUSE_FURNITURE_WRAP_V1),
        command.payload,
    ) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    let reply = authority
        .house_wrap(*admitted, command.command_id, request)
        .await?;
    let status = match reply {
        wire::Reply::Committed(_) => CommandStatus::Accepted,
        wire::Reply::Refused(_) => CommandStatus::Rejected,
    };
    Some(Dispatch::Reply(status, reply.encode()))
}

fn decode_request(selected: bool, payload: &[u8]) -> Option<wire::Request> {
    selected.then(|| wire::Request::decode(payload)).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn house_wrap_route_requires_capability_and_exact_payload() {
        let request = wire::Request {
            action: wire::Action::Wrap,
            item: [1; 16],
            expected_revision: 1,
            acl_revision: 1,
            position: [1, 2, 7],
        };
        assert_eq!(decode_request(true, &request.encode()), Some(request));
        assert!(decode_request(false, &request.encode()).is_none());
        assert!(decode_request(true, &[0; 45]).is_none());
        assert!(decode_request(true, &request.encode()[..44]).is_none());
    }
}
