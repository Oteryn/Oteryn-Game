//! Composition for MAIL_V1 through the same admitted command dispatch boundary.

use oteryn_gameplay_durability::durability::mail::{MailPostOutcome, MailPostRefusal};
use oteryn_protocol_oteryn::mail::{MailIntent, MailOutcome, decode_intent};

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
