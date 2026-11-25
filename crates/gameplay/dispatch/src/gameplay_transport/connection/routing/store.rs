//! STORE_V1 catalogue page 183 and purchase 185; connection dispatch enforces capability 154.
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, store as wire};

pub(in super::super) fn handles_command(command_type: u32) -> bool {
    matches!(
        command_type,
        wire::COMMAND_TYPE_STORE_CATALOGUE_PAGE
            | wire::COMMAND_TYPE_STORE_HISTORY_PAGE
            | wire::COMMAND_TYPE_STORE_PURCHASE_HISTORY_PAGE
            | wire::COMMAND_TYPE_STORE_PURCHASE
            | wire::COMMAND_TYPE_STORE_INBOX_CLAIM
    )
}

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

pub(in super::super) fn is_committed_claim(
    command_type: u32,
    status: CommandStatus,
    payload: &[u8],
) -> bool {
    command_type == wire::COMMAND_TYPE_STORE_INBOX_CLAIM
        && status == CommandStatus::Accepted
        && wire::decode_claim_receipt(payload)
            .is_ok_and(|receipt| receipt.result == wire::StoreClaimCode::Committed)
}
