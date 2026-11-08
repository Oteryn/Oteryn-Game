//! W23 client replay: the private `Game` line the server sends when the Character becomes a
//! citizen goes through `Session::apply_state_delta` and lands in `Session::chat_log`. The line is
//! session-owned, so the row re-encodes it with the production `chat::encode_chat_line` (the C17c
//! practice). A malformed line is refused and leaves the log and revision unchanged.
use super::*;
use crate::tests::BoxError;
use oteryn_protocol_oteryn::encode_state_delta;

struct Capture {
    line: Vec<u8>,
    expected: String,
}

fn fixture() -> Result<Capture, BoxError> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        return Ok(Capture {
            line: std::fs::read(path.join("w23-line.bin"))?,
            expected: std::fs::read_to_string(path.join("w23-expected.txt"))?,
        });
    }
    Ok(Capture {
        line: chat::encode_chat_line(&ChatLine::Private {
            speaker_name: "Game".to_owned(),
            text: "You are now a citizen of Thais.".to_owned(),
        })
        .map_err(|error| format!("{error:?}"))?,
        expected: "speaker=Game\ntext=You are now a citizen of Thais.\n".into(),
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, BoxError> {
    expected
        .lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix('='))
        })
        .ok_or_else(|| format!("w23 expected lacks {key}").into())
}

#[test]
fn e2e_w23_citizenship_message_replay() -> Result<(), BoxError> {
    block_on(async {
        let capture = fixture()?;
        let expected = capture.expected.as_str();
        let (client, peer) = chat_peer(vec![chat_domain(3, ChatRoomSet::default())], vec![]);
        let mut session = Session::admit(client, chat_admission()?).await?;
        let frame = encode_state_delta(
            1,
            41,
            STATE_DOMAIN_CHAT,
            3,
            4,
            chat::DELTA_TYPE_CHAT_LINE_V1,
            &capture.line,
        )?;
        let event = session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload())?;
        let SessionEvent::ChatLine(applied) = event else {
            return Err("citizenship line did not apply as a ChatLine event".into());
        };
        assert_eq!((applied.base_revision, applied.new_revision), (3, 4));
        assert_eq!(
            applied.value,
            ChatLine::Private {
                speaker_name: value(expected, "speaker")?.to_owned(),
                text: value(expected, "text")?.to_owned(),
            }
        );
        let kept: Vec<ChatLine> = session
            .chat_log()
            .ok_or("chat log absent")?
            .lines()
            .cloned()
            .collect();
        assert_eq!(kept, vec![applied.value.clone()]);
        let bad = encode_state_delta(
            1,
            42,
            STATE_DOMAIN_CHAT,
            4,
            5,
            chat::DELTA_TYPE_CHAT_LINE_V1,
            &[0xff],
        )?;
        assert!(
            session
                .apply_state_delta(42, decode_wire_envelope(&bad)?.payload())
                .is_err()
        );
        let after: Vec<ChatLine> = session
            .chat_log()
            .ok_or("chat log absent")?
            .lines()
            .cloned()
            .collect();
        assert_eq!(after, kept);
        drop(session);
        peer.abort();
        Ok(())
    })?
}
