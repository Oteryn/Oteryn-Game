use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use crate::durability::{
    DurabilityError,
    character_stash::container::{Quote, Request, Target},
};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::{ClientCommandView, container_batch::Request as WireRequest};
use std::num::NonZeroU64;

pub(in crate::gameplay_transport) type Pending = (NonZeroU64, Quote);

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: AdmittedSession,
    command: ClientCommandView<'_>,
    items: Option<&SessionItemView>,
    pending: &mut Option<Pending>,
) -> Option<Dispatch> {
    let request = match WireRequest::decode(command.payload) {
        Some(WireRequest::Preview {
            source,
            recursive: true,
        }) => {
            *pending = None;
            let Some(ItemKey::Instance(item)) = items.and_then(|view| view.resolve(source)) else {
                return Some(Dispatch::Unregistered);
            };
            Request::Preview(Target {
                item,
                handle: source,
            })
        }
        Some(WireRequest::Confirm { preview }) => {
            let quote = pending
                .take()
                .filter(|(id, _)| *id == preview)
                .map(|(_, quote)| quote);
            Request::Confirm { preview, quote }
        }
        _ => return Some(Dispatch::Unregistered),
    };
    let may_write = matches!(&request, Request::Confirm { quote: Some(_), .. });
    match authority
        .stash_container(admitted, command.command_id, request)
        .await
    {
        Ok(Some(result)) => {
            if let Some(quote) = result.quote {
                *pending = Some((NonZeroU64::new(command.command_id)?, quote));
            }
            Some(Dispatch::CarriedBatch(result.reply))
        }
        // A confirmation can reach the durable receipt owner with no cached quote.
        Err(DurabilityError::CommitOutcomeUnknown | DurabilityError::CommitTransactionTimeout) => {
            None
        }
        Err(_) if may_write => None,
        Ok(None) | Err(_) => Some(Dispatch::Unregistered),
    }
}
