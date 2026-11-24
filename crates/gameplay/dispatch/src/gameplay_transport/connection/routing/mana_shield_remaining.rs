use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::CommandStatus;

pub(in super::super) async fn route(
    authority: &impl FreshAdmissionAuthority,
    admitted: AdmittedSession,
    payload: &[u8],
) -> Dispatch {
    if payload.is_empty()
        && let Some(amount) = authority.mana_shield_remaining(admitted).await
    {
        Dispatch::Reply(CommandStatus::Accepted, amount.to_le_bytes().to_vec())
    } else {
        Dispatch::Reply(CommandStatus::Rejected, Vec::new())
    }
}
