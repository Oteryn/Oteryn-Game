use super::super::{AdmittedSession, ExactActorRef, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, mail::{MailIntent, decode_intent}};
use std::num::NonZeroU64;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(authority: &A, actor: ExactActorRef, admitted: &AdmittedSession, command: ClientCommandView<'_>, item_view: &Option<SessionItemView>) -> Option<Dispatch> {
    let (item, mailbox) = match decode_intent(command.payload).ok() {
        Some(MailIntent::Post { item_handle, mailbox_handle }) => {
            let item_handle = NonZeroU64::new(item_handle)?;
            let mailbox_handle = NonZeroU64::new(mailbox_handle)?;
            if let Some(outcome) = authority.replay_mail_post(
                *admitted, command.command_id, command.payload.try_into().ok()?,
            ).await.ok()? {
                return Some(Dispatch::Reply(CommandStatus::Accepted, vec![outcome as u8]));
            }
            (
                item_view.as_ref().and_then(|view| view.resolve(item_handle)),
                item_view.as_ref().and_then(|view| view.resolve(mailbox_handle)),
            )
        },
        Some(MailIntent::WriteText { item_handle, .. }) => (item_view.as_ref()?.resolve(NonZeroU64::new(item_handle)?), None),
        _ => (None, None),
    };
    let outcome=authority.mail_command(actor,*admitted,command.command_id,command.payload,item,mailbox).await?;
    Some(Dispatch::Reply(CommandStatus::Accepted, vec![outcome as u8]))
}
