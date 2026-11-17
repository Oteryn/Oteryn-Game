//! Capability 493 `CHAT_PRIVATE_ECHO_V1`: the sender of an accepted private line receives it back
//! as line kind 2 with field 12 `recipient_name`. The echo is queued only after the World relay
//! accepted the line (`ChatDisposition::Ok`), carries the sender's admitted name and the text as
//! the relay carried it, and goes out on the sender's next CHAT refresh, never to anyone else.
use super::*;
use crate::chat::EGRESS_MAX_LINES;
use oteryn_protocol_oteryn::chat::line_extended::PrivateEcho;

impl ChatRuntime {
    /// Queues the echo of an accepted private line of `session`. The queue keeps the newest
    /// `EGRESS_MAX_LINES`; an unknown session, an invalid name or text queues nothing.
    pub(in crate::gameplay_transport) fn push_private_echo(
        &self,
        session: GameSessionId,
        recipient_name: &str,
        text: &str,
    ) -> bool {
        let (Ok(recipient), Ok(text)) =
            (CharacterName::parse(recipient_name), ChatText::parse(text))
        else {
            return false;
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return false;
        };
        let Some(entry) = sessions.get_mut(&session) else {
            return false;
        };
        if entry.echoes.len() >= EGRESS_MAX_LINES {
            entry.echoes.pop_front();
        }
        entry.echoes.push_back(PrivateEcho {
            speaker_name: entry.name.clone(),
            recipient_name: recipient.as_str().to_owned(),
            text: text.as_str().to_owned(),
        });
        true
    }

    /// The oldest undelivered echo of `session`.
    pub(in crate::gameplay_transport) fn next_private_echo(
        &self,
        session: GameSessionId,
    ) -> Option<PrivateEcho> {
        self.sessions
            .lock()
            .ok()?
            .get_mut(&session)?
            .echoes
            .pop_front()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures")]
mod tests {
    use super::*;

    const BASE: SemanticTimeMicros = SemanticTimeMicros::from_micros(1_000_000);

    fn session_of(tag: u8) -> GameSessionId {
        GameSessionId::decode(&[
            0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
        ])
        .expect("session")
    }

    #[test]
    fn an_accepted_private_line_echoes_to_its_sender_only_with_the_recipient() {
        let chat = ChatRuntime::default();
        let (sender, other) = (session_of(0x41), session_of(0x42));
        chat.admit(sender, "Aldric", BASE);
        chat.admit(other, "Bryn", BASE);
        assert!(chat.push_private_echo(sender, "Bryn", "  hello  "));
        assert_eq!(
            chat.next_private_echo(sender),
            Some(PrivateEcho {
                speaker_name: "Aldric".into(),
                recipient_name: "Bryn".into(),
                text: "hello".into(),
            })
        );
        assert_eq!(chat.next_private_echo(sender), None);
        assert_eq!(chat.next_private_echo(other), None);
        // An unknown session, an empty text or a name that is no Character name queue nothing.
        assert!(!chat.push_private_echo(session_of(0x43), "Bryn", "hello"));
        assert!(!chat.push_private_echo(sender, "Bryn", "   "));
        assert!(!chat.push_private_echo(sender, "", "hello"));
        assert_eq!(chat.next_private_echo(sender), None);
    }

    #[test]
    fn the_echo_queue_is_bounded_and_dropped_on_reconnect() {
        let chat = ChatRuntime::default();
        let sender = session_of(0x44);
        chat.admit(sender, "Aldric", BASE);
        for index in 0..=EGRESS_MAX_LINES {
            assert!(chat.push_private_echo(sender, "Bryn", &format!("line {index}")));
        }
        // The oldest echo gave way to the newest.
        assert_eq!(
            chat.next_private_echo(sender).map(|echo| echo.text),
            Some("line 1".to_owned())
        );
        let mut left = 1;
        while chat.next_private_echo(sender).is_some() {
            left += 1;
        }
        assert_eq!(left, EGRESS_MAX_LINES);
        // A replacement snapshot drops undelivered echoes, as it drops undelivered lines.
        assert!(chat.push_private_echo(sender, "Bryn", "before"));
        chat.refresh(sender);
        assert_eq!(chat.next_private_echo(sender), None);
        assert!(chat.push_private_echo(sender, "Bryn", "again"));
        chat.admit(sender, "Aldric", BASE);
        assert_eq!(chat.next_private_echo(sender), None);
        assert!(chat.push_private_echo(sender, "Bryn", "last"));
        chat.forget(sender);
        assert_eq!(chat.next_private_echo(sender), None);
    }
}
