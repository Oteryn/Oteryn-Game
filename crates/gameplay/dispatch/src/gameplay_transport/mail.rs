//! Composition for MAIL_V1 through the same admitted command dispatch boundary.

use super::{ComposedFreshAdmission, connection::AdmittedSession, item_view::ItemKey};
use oteryn_gameplay_durability::durability::mail::{MailPostOutcome, MailPostRefusal};
use oteryn_protocol_oteryn::mail::{MailIntent, MailOutcome, decode_intent};

macro_rules! authority_methods {
    () => {
        fn mail_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command_id: u64,
            _payload: &[u8],
            _item: Option<$crate::gameplay_transport::item_view::ItemKey>,
            _mailbox: Option<$crate::gameplay_transport::item_view::ItemKey>,
        ) -> impl std::future::Future<Output = Option<oteryn_protocol_oteryn::mail::MailOutcome>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn mail_command(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            command_id: u64,
            payload: &[u8],
            item: Option<$crate::gameplay_transport::item_view::ItemKey>,
            mailbox: Option<$crate::gameplay_transport::item_view::ItemKey>,
        ) -> Option<oteryn_protocol_oteryn::mail::MailOutcome> {
            $crate::gameplay_transport::mail::execute(
                self, admitted, command_id, payload, item, mailbox,
            )
            .await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    item: Option<ItemKey>,
    mailbox: Option<ItemKey>,
) -> Option<MailOutcome> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(u32::from(oteryn_protocol_oteryn::mail::CAPABILITY_MAIL_V1))
    {
        return Some(MailOutcome::Rejected);
    }
    let (
        Some(fence),
        Ok(MailIntent::Post { .. }),
        Some(ItemKey::Instance(item)),
        Some(ItemKey::MapBase { placement_key, .. }),
    ) = (admitted.item_fence, decode_intent(payload), item, mailbox)
    else {
        return Some(MailOutcome::Rejected);
    };
    owner
        .root
        .post_admitted_mail(
            owner.character,
            owner.holder,
            fence,
            command_id,
            item,
            placement_key,
        )
        .await
        .ok()
        .map(wire_post_outcome)
}

/// The transport owns decoding only. The implementation owns handle reach,
/// Content and session fencing, replay, and the atomic database transaction.
pub trait MailCommandOwner {
    fn post(&mut self, item_handle: u64, mailbox_handle: u64) -> MailPostOutcome;
    fn write_text(&mut self, item_handle: u64, text: &str) -> MailOutcome;
}

pub fn dispatch_mail(owner: &mut impl MailCommandOwner, payload: &[u8]) -> MailOutcome {
    match decode_intent(payload) {
        Ok(MailIntent::Post {
            item_handle,
            mailbox_handle,
        }) => wire_post_outcome(owner.post(item_handle, mailbox_handle)),
        Ok(MailIntent::WriteText { item_handle, text }) => owner.write_text(item_handle, &text),
        Err(_) => MailOutcome::Rejected,
    }
}

fn wire_post_outcome(outcome: MailPostOutcome) -> MailOutcome {
    match outcome {
        MailPostOutcome::Posted | MailPostOutcome::AlreadyPosted => MailOutcome::Posted,
        MailPostOutcome::Refused(refusal) => match refusal {
            MailPostRefusal::UnknownRecipient => MailOutcome::UnknownRecipient,
            MailPostRefusal::NotMailable => MailOutcome::NotMailable,
            MailPostRefusal::RecipientInboxFull => MailOutcome::RecipientInboxFull,
            MailPostRefusal::Exhausted => MailOutcome::Exhausted,
            MailPostRefusal::ReplayConflict => MailOutcome::Rejected,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::mail::{MailWireError, encode_intent};

    struct Owner(Vec<String>);
    impl MailCommandOwner for Owner {
        fn post(&mut self, item: u64, mailbox: u64) -> MailPostOutcome {
            self.0.push(format!("post:{item}:{mailbox}"));
            MailPostOutcome::Posted
        }
        fn write_text(&mut self, item: u64, text: &str) -> MailOutcome {
            self.0.push(format!("write:{item}:{text}"));
            MailOutcome::TextWritten
        }
    }
    #[test]
    fn every_post_outcome_maps_through_dispatch() -> Result<(), MailWireError> {
        struct ResultOwner(MailPostOutcome);
        impl MailCommandOwner for ResultOwner {
            fn post(&mut self, _: u64, _: u64) -> MailPostOutcome {
                self.0
            }
            fn write_text(&mut self, _: u64, _: &str) -> MailOutcome {
                unreachable!()
            }
        }
        let payload = encode_intent(&MailIntent::Post {
            item_handle: 1,
            mailbox_handle: 2,
        })?;
        for (outcome, expected) in [
            (MailPostOutcome::Posted, MailOutcome::Posted),
            (MailPostOutcome::AlreadyPosted, MailOutcome::Posted),
            (
                MailPostOutcome::Refused(MailPostRefusal::UnknownRecipient),
                MailOutcome::UnknownRecipient,
            ),
            (
                MailPostOutcome::Refused(MailPostRefusal::NotMailable),
                MailOutcome::NotMailable,
            ),
            (
                MailPostOutcome::Refused(MailPostRefusal::RecipientInboxFull),
                MailOutcome::RecipientInboxFull,
            ),
            (
                MailPostOutcome::Refused(MailPostRefusal::Exhausted),
                MailOutcome::Exhausted,
            ),
            (
                MailPostOutcome::Refused(MailPostRefusal::ReplayConflict),
                MailOutcome::Rejected,
            ),
        ] {
            assert_eq!(dispatch_mail(&mut ResultOwner(outcome), &payload), expected);
        }
        Ok(())
    }

    #[test]
    fn composes_both_real_commands_and_refuses_malformed_input() -> Result<(), MailWireError> {
        let mut owner = Owner(Vec::new());
        let post = encode_intent(&MailIntent::Post {
            item_handle: 3,
            mailbox_handle: 4,
        })?;
        let write = encode_intent(&MailIntent::WriteText {
            item_handle: 3,
            text: "Ada".into(),
        })?;
        assert_eq!(dispatch_mail(&mut owner, &post), MailOutcome::Posted);
        assert_eq!(dispatch_mail(&mut owner, &write), MailOutcome::TextWritten);
        assert_eq!(dispatch_mail(&mut owner, &[]), MailOutcome::Rejected);
        assert_eq!(owner.0, ["post:3:4", "write:3:Ada"]);
        Ok(())
    }
}
