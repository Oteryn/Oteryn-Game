use super::super::{AdmittedSession, ExactActorRef, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, mail::{MailIntent, decode_intent}};
use std::num::NonZeroU64;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(authority: &A, actor: ExactActorRef, admitted: &AdmittedSession, command: ClientCommandView<'_>, item_view: &Option<SessionItemView>) -> Option<Dispatch> {
    let (item, mailbox) = match decode_intent(command.payload).ok() {
        Some(MailIntent::Post { item_handle, mailbox_handle }) => (item_view.as_ref()?.resolve(NonZeroU64::new(item_handle)?), item_view.as_ref()?.resolve(NonZeroU64::new(mailbox_handle)?)),
        _ => (None, None),
    };
    let outcome=authority.mail_command(actor,*admitted,command.command_id,command.payload,item,mailbox).await?;
    Some(Dispatch::Reply(CommandStatus::Accepted, vec![outcome as u8]))
}
