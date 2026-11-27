//! S18 replays complete inbound wire recordings, including each live admission snapshot.
use super::*;
use oteryn_protocol_oteryn::{decode_command_result, decode_state_delta, decode_wire_envelope};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

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

struct RecordedInput<'a>(&'a [u8]);

impl AsyncRead for RecordedInput<'_> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let length = buffer.remaining().min(self.0.len());
        buffer.put_slice(&self.0[..length]);
        self.0 = &self.0[length..];
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for RecordedInput<'_> {
    // Admission writes the same client bootstrap API into this offline input. No writes can
    // supply any inbound evidence.
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

async fn verify_envelopes(
    mut wire: &[u8],
    session: &str,
    envelopes: &[serde_json::Value],
) -> Result<(), BoxError> {
    for captured in envelopes {
        let frame = read_frame(&mut wire).await?;
        let envelope = decode_wire_envelope(&frame)?;
        let (command_id, status) = if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            (Some(result.command_id), Some(result.status as u32))
        } else {
            (None, None)
        };
        let expected = serde_json::json!({
            "command_id": command_id,
            "game_session_id": session,
            "payload": envelope.payload(),
            "server_sequence": envelope.server_sequence(),
            "status": status,
        });
        if captured != &expected {
            return Err(format!(
                "S18 envelope disagrees with original frame at sequence {}",
                envelope.server_sequence()
            )
            .into());
        }
    }
    if !wire.is_empty() {
        return Err("S18 capture omits an original server frame".into());
    }
    Ok(())
}

async fn replay(
    capture: &[u8],
    expected: &[ChatLine],
    envelopes: &[serde_json::Value],
) -> Result<(JoinSnapshot, ActorPosition), BoxError> {
    let header_end = capture
        .iter()
        .position(|byte| *byte == b'\n')
        .ok_or("S18 recording missing game session and command provenance")?;
    let (provenance, wire) = (&capture[..header_end], &capture[header_end + 1..]);
    let mut claims = std::str::from_utf8(provenance)?.split_ascii_whitespace();
    let claimed_session = claims.next().ok_or("S18 recording missing game session")?;
    let mut claimed_commands = claims.peekable();
    if claimed_commands.peek().is_none() {
        return Err("S18 recording missing command provenance".into());
    }
    let mut session = Session::admit(
        RecordedInput(wire),
        admission_with(CLIENT_SUPPORTED_CAPABILITIES)?,
    )
    .await?;
    let joined = session.join_snapshot().clone();
    let actual_session: String = joined
        .game_session_id
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if claimed_session != actual_session {
        return Err("S18 recorded game session mismatch".into());
    }
    let session_uuid = format!(
        "{}-{}-{}-{}-{}",
        &actual_session[..8],
        &actual_session[8..12],
        &actual_session[12..16],
        &actual_session[16..20],
        &actual_session[20..],
    );
    verify_envelopes(wire, &session_uuid, envelopes).await?;
    assert!(
        session
            .chat_log()
            .ok_or("S18 chat snapshot absent")?
            .is_empty()
    );
    let mut seen = 0;
    let mut previous_command = 0;
    let mut malformed_checked = [false; 2];
    while !session.stream.0.is_empty() {
        let frame = read_frame(&mut session.stream).await?;
        let envelope = decode_wire_envelope(&frame)?;
        envelope.validate(Direction::ServerToClient, true)?;
        session.check_generation(&envelope)?;
        if let Some(error) = session.terminal_protocol_error(&envelope)? {
            return Err(error.into());
        }
        match envelope.message_type() {
            MessageType::LivenessProbe => {
                session.answer_probe(envelope.payload()).await?;
                continue;
            }
            MessageType::SnapshotBegin => {
                session.replace_snapshot(envelope.payload()).await?;
                session.take_events();
                println!(
                    "S18_CAPTURED_REPLACEMENT sequence={}",
                    session.last_server_sequence()
                );
                continue;
            }
            MessageType::CommandResult | MessageType::StateDelta => {}
            actual => return Err(format!("S18 unexpected recorded frame: {actual:?}").into()),
        }
        let sequence = session.next_sequence(&envelope)?;
        if envelope.message_type() == MessageType::CommandResult {
            let outcome = decode_command_result(envelope.payload())?;
            let receipt = format!("{}:{sequence}", outcome.command_id);
            if claimed_commands.next() != Some(receipt.as_str()) {
                return Err(
                    format!("S18 recorded command or server sequence mismatch: {receipt}").into(),
                );
            }
            assert_eq!(
                outcome.command_id,
                previous_command + 1,
                "S18 first-attempt command ID"
            );
            assert_eq!(
                outcome.status,
                CommandStatus::Accepted,
                "S18 first-attempt result"
            );
            previous_command = outcome.command_id;
            if claimed_commands.peek().is_none() {
                assert_eq!(
                    oteryn_protocol_oteryn::logout::decode_logout_result(outcome.payload)
                        .map_err(SessionError::Logout)?,
                    oteryn_protocol_oteryn::logout::LogoutResult::Accepted,
                    "S18 final command must be an accepted logout",
                );
            }
        }
        if envelope.message_type() == MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload())?;
            if delta.domain_id == STATE_DOMAIN_CHAT
                && delta.delta_type == oteryn_protocol_oteryn::chat::DELTA_TYPE_CHAT_LINE_V1
                && let Some(ChatLine::Local { mode, .. }) = expected.get(seen)
            {
                let probe = match mode {
                    ChatSpeechMode::Say => 0,
                    ChatSpeechMode::Whisper => 1,
                    _ => return Err("S18 replay expected say or whisper".into()),
                };
                if !malformed_checked[probe] {
                    let malformed = oteryn_protocol_oteryn::encode_state_delta(
                        envelope.connection_generation(),
                        sequence,
                        delta.domain_id,
                        delta.base_revision,
                        delta.new_revision,
                        delta.delta_type,
                        &delta.payload[..delta.payload.len() - 1],
                    )?;
                    let malformed = decode_wire_envelope(&malformed)?;
                    assert_eq!(
                        decode_state_delta(malformed.payload())?.base_revision,
                        session.chat_revision
                    );
                    assert!(
                        matches!(
                            session.apply_state_delta(sequence, malformed.payload()),
                            Err(SessionError::Chat(_))
                        ),
                        "S18 malformed inner chat must reach the chat codec"
                    );
                    assert_eq!(session.chat_revision, delta.base_revision);
                    assert!(
                        session
                            .chat_log()
                            .ok_or("S18 chat log absent")?
                            .lines()
                            .eq(&expected[..seen])
                    );
                    println!(
                        "S18_CHAT_CODEC_REJECTION mode={mode:?} sequence={sequence} revision={}",
                        delta.base_revision
                    );
                    malformed_checked[probe] = true;
                }
            }
            let event = session.apply_state_delta(sequence, envelope.payload())?;
            if let SessionEvent::ChatLine(applied) = event {
                assert_eq!(delta.domain_id, STATE_DOMAIN_CHAT);
                assert_eq!(
                    delta.base_revision.checked_add(1),
                    Some(delta.new_revision),
                    "S18 chat revision must advance one step"
                );
                assert_eq!(
                    (applied.base_revision, applied.new_revision),
                    (delta.base_revision, delta.new_revision),
                );
                assert_eq!(expected.get(seen), Some(&applied.value), "S18 line {seen}");
                seen += 1;
            }
        }
    }
    if claimed_commands.next().is_some() {
        return Err("S18 missing recorded command result".into());
    }
    assert_eq!(seen, expected.len(), "S18 recorded chat line count");
    assert_eq!(
        malformed_checked,
        [!expected.is_empty(); 2],
        "S18 chat codec check"
    );
    assert!(
        session
            .chat_log()
            .ok_or("S18 chat log absent")?
            .lines()
            .eq(expected)
    );

    Ok((joined, session.world_spatial().actor_position))
}

#[test]
#[ignore = "live qualification replay"]
fn e2e_s18_say_and_whisper_echo_replay() -> Result<(), BoxError> {
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("S18 live capture directory required")?;
    let root = Path::new(&root);
    let run = root
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .filter(|name| name.starts_with("captures.") && name.len() == 15)
        .ok_or("S18 replay requires a fresh cloud capture directory")?;
    let checkout = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !checkout.status.success() {
        return Err("S18 replay head unavailable".into());
    }
    let head = String::from_utf8(checkout.stdout)?;
    let binding = format!("{} {run}\n", head.trim());
    let metadata = String::from_utf8(read_bound(root, "s18-expected.txt", &binding)?)?;
    let identity = value(&metadata, "speaker")?;
    if identity.len() != 32 || !identity.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("S18 capture speaker must be a 16-byte identity".into());
    }
    let mut bytes = [0; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&identity[index * 2..index * 2 + 2], 16)?;
    }
    let speaker = ChatSpeaker {
        identity: bytes,
        generation: value(&metadata, "generation")?.parse()?,
    };
    let position = ActorPosition {
        x: value(&metadata, "x")?.parse()?,
        y: value(&metadata, "y")?.parse()?,
        floor: value(&metadata, "floor")?.parse()?,
    };
    let name = value(&metadata, "name")?;
    assert_eq!(name, "Packet Two");
    let line = |mode, text: String, position| ChatLine::Local {
        speaker,
        speaker_name: name.into(),
        mode,
        text,
        position,
    };
    let mut speaker_lines = Vec::new();
    let mut listener_lines = Vec::new();
    for label in [
        "near",
        "boundary-whisper-two",
        "boundary-whisper-two-x",
        "boundary-whisper-two-y",
        "boundary-north",
        "outside-north",
        "boundary-west",
        "outside-west",
        "boundary-south",
        "outside-south",
        "boundary-east",
        "outside-east",
        "boundary-diagonal",
        "other-floor",
        "adjacent-other-floor",
    ] {
        let (dx, dy) = match label {
            "boundary-whisper-two-x" => (1, 0),
            "boundary-whisper-two-y" => (-1, 0),
            "adjacent-other-floor" => (7, 3),
            "boundary-north" => (1, -1),
            "boundary-south" => (-1, 1),
            "boundary-west" => (0, 1),
            "boundary-east" => (0, 2),
            "boundary-diagonal" => (0, -1),
            _ => (0, 0),
        };
        let at = ActorPosition {
            x: position.x + dx,
            y: position.y + dy,
            ..position
        };
        for (mode, suffix) in [
            (ChatSpeechMode::Say, "say"),
            (ChatSpeechMode::Whisper, "whisper"),
        ] {
            let text = format!("s18-{label}-{suffix}-{run}");
            speaker_lines.push(line(mode, text.clone(), at));
            if label == "near" {
                listener_lines.push(line(mode, text, at));
            } else if label.starts_with("boundary-") {
                let heard = if mode == ChatSpeechMode::Whisper {
                    "pspsps".into()
                } else {
                    text
                };
                listener_lines.push(line(mode, heard, at));
            }
        }
    }
    let balcony_identity = value(&metadata, "balcony_speaker")?;
    if balcony_identity.len() != 32
        || !balcony_identity
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("S18 balcony speaker must be a 16-byte identity".into());
    }
    let mut balcony_bytes = [0; 16];
    for (index, byte) in balcony_bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&balcony_identity[index * 2..index * 2 + 2], 16)?;
    }
    let balcony_speaker = ChatSpeaker {
        identity: balcony_bytes,
        generation: value(&metadata, "balcony_generation")?.parse()?,
    };
    let balcony_position = ActorPosition {
        x: value(&metadata, "balcony_x")?.parse()?,
        y: value(&metadata, "balcony_y")?.parse()?,
        floor: value(&metadata, "balcony_floor")?.parse()?,
    };
    for label in ["below-floor", "adjacent-below-floor"] {
        for (mode, suffix) in [
            (ChatSpeechMode::Say, "say"),
            (ChatSpeechMode::Whisper, "whisper"),
        ] {
            listener_lines.push(ChatLine::Local {
                speaker: balcony_speaker,
                speaker_name: "Packet One".into(),
                mode,
                text: format!("s18-{label}-{suffix}-{run}"),
                position: balcony_position,
            });
        }
    }
    block_on(async {
        for (role, envelope_file, relog_file, expected) in [
            (
                "speaker",
                "s18-speaker.json",
                "s18-speaker-relog.json",
                speaker_lines.as_slice(),
            ),
            (
                "listener",
                "s18-listener.json",
                "s18-listener-relog.json",
                listener_lines.as_slice(),
            ),
        ] {
            let envelopes: Vec<serde_json::Value> =
                serde_json::from_slice(&std::fs::read(root.join(envelope_file))?)?;
            let relog_envelopes: Vec<serde_json::Value> =
                serde_json::from_slice(&std::fs::read(root.join(relog_file))?)?;
            let wire = read_bound(root, &format!("s18-{role}.wire"), &binding)?;
            let (initial, after) = replay(&wire, expected, &envelopes).await?;
            let relog = read_bound(root, &format!("s18-{role}-relog.wire"), &binding)?;
            let (fresh, _) = replay(&relog, &[], &relog_envelopes).await?;
            assert_ne!(initial.game_session_id, fresh.game_session_id);
            assert_eq!(after, fresh.world_spatial.actor_position);
            if role == "listener" {
                assert_ne!(initial.world_spatial.actor_position, after);
            }
        }
        Ok(())
    })?
}
