//! S18 replay: the say and whisper echoes applied through `Session::apply_state_delta` (the
//! capability-7 chat-line decode in `chat.rs`) and exposed by `Session::chat_log`, the log
//! `DevClientSession` and the chat panel read.
use super::*;
use oteryn_protocol_oteryn::{decode_wire_envelope, encode_state_delta};
use std::path::Path;
use tokio::io::DuplexStream;

const SELECTED: &[u32] = &[7, 13];

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, BoxError> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn read_bound(root: &Path, file: &str, binding: &str) -> Result<Vec<u8>, BoxError> {
    let bytes = std::fs::read(root.join(file))?;
    bytes
        .strip_prefix(binding.as_bytes())
        .filter(|payload| !payload.is_empty())
        .map(<[u8]>::to_vec)
        .ok_or_else(|| format!("{file}: capture run/head mismatch or empty payload").into())
}

fn capture_binding(head: &str, run: &str) -> Result<String, BoxError> {
    if head.len() != 40
        || run.len() != 32
        || !head
            .bytes()
            .chain(run.bytes())
            .all(|b| b.is_ascii_hexdigit())
    {
        return Err("S18 replay requires a Git head and unique run token".into());
    }
    Ok(format!("{head} {run}\n"))
}

async fn admitted() -> Result<(Session<DuplexStream>, DuplexStream), BoxError> {
    let (client, mut server) = tokio::io::duplex(1 << 16);
    let peer = tokio::spawn(async move {
        join_peer_with(
            &mut server,
            SELECTED,
            SELECTED,
            &[],
            (1, spatial(0)),
            &[(
                STATE_DOMAIN_CHAT,
                3,
                chat::SNAPSHOT_TYPE_CHAT_V1,
                chat::encode_chat_rooms(ChatRoomSet::default()),
            )],
        )
        .await?;
        Ok::<_, BoxError>(server)
    });
    let session = Session::admit(client, admission_with(SELECTED)?).await?;
    Ok((session, peer.await??))
}

fn delta(
    session: &mut Session<DuplexStream>,
    sequence: u64,
    base: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    let frame = encode_state_delta(
        1,
        sequence,
        STATE_DOMAIN_CHAT,
        base,
        base + 1,
        chat::DELTA_TYPE_CHAT_LINE_V1,
        payload,
    )?;
    session.apply_state_delta(sequence, decode_wire_envelope(&frame)?.payload())
}

#[test]
#[ignore = "live-only replay, run by tools/qualification/e2e_all/run_s18.sh"]
fn e2e_s18_say_and_whisper_echo_replay() -> Result<(), BoxError> {
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("S18 live capture directory required")?;
    let root = Path::new(&root);
    let head = std::env::var("S18_CAPTURE_HEAD")?;
    let run = std::env::var("S18_CAPTURE_RUN")?;
    let binding = capture_binding(&head, &run)?;
    let expected = String::from_utf8(read_bound(root, "s18-expected.txt", &binding)?)?;
    let speaker = ChatSpeaker {
        identity: {
            let hex = value(&expected, "speaker")?;
            if hex.len() != 32 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("S18 capture speaker must be a 16-byte identity".into());
            }
            let mut bytes = [0_u8; 16];
            for (index, byte) in bytes.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)?;
            }
            bytes
        },
        generation: value(&expected, "generation")?.parse()?,
    };
    let position = ActorPosition {
        x: value(&expected, "x")?.parse()?,
        y: value(&expected, "y")?.parse()?,
        floor: value(&expected, "floor")?.parse()?,
    };
    let name = value(&expected, "name")?;
    assert_eq!(name, "Packet Two");

    for role in ["speaker", "listener"] {
        let whisper = if role == "listener" {
            "pspsps".into()
        } else {
            format!("s18-private-{run}")
        };
        let mut lines = vec![
            (ChatSpeechMode::Say, format!("s18-say-{run}")),
            (ChatSpeechMode::Whisper, format!("s18-whisper-{run}")),
            (ChatSpeechMode::Say, format!("s18-boundary-{run}")),
            (ChatSpeechMode::Whisper, whisper),
        ];
        if role == "speaker" {
            lines.push((ChatSpeechMode::Say, format!("s18-outside-{run}")));
        }
        block_on(async {
            let (mut session, _server) = admitted().await?;
            assert_eq!(
                session.chat_log().ok_or("chat log absent")?.lines().count(),
                0
            );
            let mut expected_log = Vec::new();
            let mut last_payload = Vec::new();
            for (index, (mode, text)) in lines.into_iter().enumerate() {
                let base = 3 + index as u64;
                let payload = read_bound(root, &format!("s18-{role}-{index}.bin"), &binding)?;
                let line = ChatLine::Local {
                    speaker,
                    speaker_name: name.into(),
                    mode,
                    text,
                    position,
                };
                let event = delta(&mut session, 41 + index as u64, base, &payload)?;
                assert!(
                    matches!(&event, SessionEvent::ChatLine(applied)
                    if (applied.base_revision, applied.new_revision) == (base, base + 1) && applied.value == line),
                    "{role} index {index}: {event:?}"
                );
                expected_log.push(line);
                last_payload = payload;
            }
            assert!(
                session
                    .chat_log()
                    .ok_or("chat log absent")?
                    .lines()
                    .eq(&expected_log)
            );

            // A truncated line changes neither the applied log nor the revision.
            let before_revision = session.chat_revision;
            assert!(
                delta(
                    &mut session,
                    49,
                    3 + expected_log.len() as u64,
                    &last_payload[..last_payload.len() - 1]
                )
                .is_err()
            );
            assert!(
                session
                    .chat_log()
                    .ok_or("chat log absent")?
                    .lines()
                    .eq(&expected_log)
            );
            assert_eq!(session.chat_revision, before_revision);
            Ok::<(), BoxError>(())
        })??;
    }
    Ok(())
}
