use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::foundation::CharacterId;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, wheel_presentation as wire};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    a: &A,
    s: &AdmittedSession,
    c: ClientCommandView<'_>,
) -> Result<Dispatch, ()> {
    Ok(match c.command_type {
        wire::COMMAND_TYPE_WHEEL_INSPECT_CHARACTER => {
            // Crystal refuses another Character's Wheel regardless of visibility.
            Dispatch::Reply(
                CommandStatus::Rejected,
                if CharacterId::decode(c.payload).is_ok() {
                    wire::WHEEL_INSPECT_REFUSAL.to_vec()
                } else {
                    Vec::new()
                },
            )
        }
        wire::COMMAND_TYPE_WHEEL_GIFT_OF_LIFE_STATUS if c.payload.is_empty() => {
            match a.wheel_gift_status(*s).await? {
                Some(status) => Dispatch::Reply(CommandStatus::Accepted, status.to_vec()),
                None => Dispatch::Reply(CommandStatus::Rejected, Vec::new()),
            }
        }
        wire::COMMAND_TYPE_WHEEL_GIFT_OF_LIFE_STATUS => {
            Dispatch::Reply(CommandStatus::Rejected, Vec::new())
        }
        _ => route_presentation(a, s, c.payload).await,
    })
}

async fn route_presentation<A: FreshAdmissionAuthority>(
    a: &A,
    s: &AdmittedSession,
    payload: &[u8],
) -> Dispatch {
    let Some(r) = wire::Request::decode(payload) else {
        return Dispatch::Reply(CommandStatus::Rejected, vec![]);
    };
    match a.wheel_presentation_query(*s, r).await {
        Some(p) => Dispatch::Reply(CommandStatus::Accepted, p),
        None => Dispatch::Reply(CommandStatus::Rejected, vec![]),
    }
}
