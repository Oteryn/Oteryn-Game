use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use oteryn_protocol_oteryn::item_inscription::{
    COMMAND_TYPE_ITEM_INSCRIPTION_ATTRIBUTION, Request,
};
use std::num::NonZeroU64;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    item_view: &Option<SessionItemView>,
) -> Dispatch {
    let item = Request::decode(command.payload).and_then(|request| {
        item_view
            .as_ref()?
            .resolve(NonZeroU64::new(request.item_handle)?)
    });
    let attributed = command.command_type == COMMAND_TYPE_ITEM_INSCRIPTION_ATTRIBUTION;
    match item {
        Some(item) => match authority
            .item_inscription_read(*admitted, Some(item), attributed)
            .await
        {
            Some(bytes) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes),
            None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
        },
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}
