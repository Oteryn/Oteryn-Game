//! Normalizes only the session's own live item handle; partner refs remain exact generations.
use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use oteryn_protocol_oteryn::{
    ClientCommandView, CommandStatus,
    player_trade::{self as wire, Intent, ResultCode},
};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: AdmittedSession,
    command: ClientCommandView<'_>,
    view: Option<&SessionItemView>,
) -> Option<Dispatch> {
    let result = match wire::decode_intent(command.payload) {
        Err(_) => ResultCode::Rejected,
        Ok(intent) => {
            let item = match intent {
                Intent::Offer { item, .. } => {
                    std::num::NonZeroU64::new(item).and_then(|handle| view?.resolve(handle))
                }
                _ => None,
            };
            authority
                .player_trade_command(admitted, command.command_id, intent, item)
                .await?
        }
    };
    Some(Dispatch::Reply(
        if result == ResultCode::Rejected {
            CommandStatus::Rejected
        } else {
            CommandStatus::Accepted
        },
        result.payload(),
    ))
}
