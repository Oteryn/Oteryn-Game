//! Composition for MAIL_V1 through the same admitted command dispatch boundary.

use oteryn_protocol_oteryn::mail::{MailIntent, MailOutcome, decode_intent};

/// The transport owns decoding only. The implementation owns handle reach,
/// Content and session fencing, replay, and the atomic database transaction.
pub trait MailCommandOwner {
    fn post(&mut self, item_handle: u64, mailbox_handle: u64) -> MailOutcome;
    fn write_text(&mut self, item_handle: u64, text: &str) -> MailOutcome;
}

pub fn dispatch_mail(owner: &mut impl MailCommandOwner, payload: &[u8]) -> MailOutcome {
    match decode_intent(payload) {
        Ok(MailIntent::Post {
            item_handle,
            mailbox_handle,
        }) => owner.post(item_handle, mailbox_handle),
        Ok(MailIntent::WriteText { item_handle, text }) => owner.write_text(item_handle, &text),
        Err(()) => MailOutcome::Rejected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::mail::encode_intent;

    struct Owner(Vec<String>);
    impl MailCommandOwner for Owner {
        fn post(&mut self, item: u64, mailbox: u64) -> MailOutcome {
            self.0.push(format!("post:{item}:{mailbox}"));
            MailOutcome::Posted
        }
        fn write_text(&mut self, item: u64, text: &str) -> MailOutcome {
            self.0.push(format!("write:{item}:{text}"));
            MailOutcome::TextWritten
        }
    }
    #[test]
    fn composes_both_real_commands_and_refuses_malformed_input() {
        let mut owner = Owner(Vec::new());
        let post = encode_intent(&MailIntent::Post {
            item_handle: 3,
            mailbox_handle: 4,
        })
        .unwrap();
        let write = encode_intent(&MailIntent::WriteText {
            item_handle: 3,
            text: "Ada".into(),
        })
        .unwrap();
        assert_eq!(dispatch_mail(&mut owner, &post), MailOutcome::Posted);
        assert_eq!(dispatch_mail(&mut owner, &write), MailOutcome::TextWritten);
        assert_eq!(dispatch_mail(&mut owner, &[]), MailOutcome::Rejected);
        assert_eq!(owner.0, ["post:3:4", "write:3:Ada"]);
    }
}
