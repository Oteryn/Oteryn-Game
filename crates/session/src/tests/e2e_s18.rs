//! S18 replays complete inbound wire recordings, including each live admission snapshot.
use super::*;
use oteryn_protocol_oteryn::{decode_state_delta, decode_wire_envelope};
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

async fn replay(
    wire: &[u8],
    expected: &[ChatLine],
) -> Result<(JoinSnapshot, ActorPosition), BoxError> {
    let mut session = Session::admit(
        RecordedInput(wire),
        admission_with(CLIENT_SUPPORTED_CAPABILITIES)?,
    )
    .await?;
    let joined = session.join_snapshot().clone();
    assert!(
        session
            .chat_log()
            .ok_or("S18 chat snapshot absent")?
            .is_empty()
    );
    let mut seen = 0;
    let mut last_line = None;
    let mut previous_sequence = session.last_server_sequence();
    while !session.stream.0.is_empty() {
        let frame = read_frame(&mut session.stream).await?;
        let envelope = decode_wire_envelope(&frame)?;
        envelope.validate(Direction::ServerToClient, true)?;
        session.check_generation(&envelope)?;
        let sequence = envelope.server_sequence();
        if sequence != 0 {
            assert!(sequence > previous_sequence, "S18 wire sequence regressed");
            previous_sequence = sequence;
        }
        if envelope.message_type() == MessageType::StateDelta {
            let event = session.apply_state_delta(sequence, envelope.payload())?;
            if let SessionEvent::ChatLine(applied) = event {
                let delta = decode_state_delta(envelope.payload())?;
                assert_eq!(delta.domain_id, STATE_DOMAIN_CHAT);
                assert_eq!(
                    (applied.base_revision, applied.new_revision),
                    (delta.base_revision, delta.new_revision),
                );
                assert_eq!(expected.get(seen), Some(&applied.value), "S18 line {seen}");
                seen += 1;
                last_line = Some((sequence, envelope.payload().to_vec()));
            }
        }
    }
    assert_eq!(seen, expected.len(), "S18 recorded chat line count");
    assert!(
        session
            .chat_log()
            .ok_or("S18 chat log absent")?
            .lines()
            .eq(expected)
    );

    // Only the negative assertion mutates bytes. It truncates the last real delta payload
    // without creating a new envelope, and must leave both the log and revision unchanged.
    if let Some((sequence, payload)) = last_line {
        let revision = session.chat_revision;
        assert!(
            session
                .apply_state_delta(sequence, &payload[..payload.len() - 1])
                .is_err()
        );
        assert_eq!(session.chat_revision, revision);
        assert!(
            session
                .chat_log()
                .ok_or("S18 chat log absent")?
                .lines()
                .eq(expected)
        );
    }
    Ok((joined, session.world_spatial().actor_position))
}

#[test]
#[ignore = "live-only replay, run by tools/qualification/e2e_all/run_s18.sh"]
fn e2e_s18_say_and_whisper_echo_replay() -> Result<(), BoxError> {
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("S18 live capture directory required")?;
    let root = Path::new(&root);
    let head = std::env::var("S18_CAPTURE_HEAD")?;
    let run = std::env::var("S18_CAPTURE_RUN")?;
    if head.len() != 40
        || run.len() != 32
        || !head
            .bytes()
            .chain(run.bytes())
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("S18 replay requires a Git head and unique run token".into());
    }
    let checkout = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !checkout.status.success() || String::from_utf8(checkout.stdout)?.trim() != head {
        return Err("S18 replay head differs from the checkout".into());
    }
    let binding = format!("{head} {run}\n");
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
    let say = format!("s18-say-{run}");
    let whisper = format!("s18-whisper-{run}");
    let boundary = format!("s18-boundary-{run}");
    let private = format!("s18-private-{run}");
    let outside = format!("s18-outside-{run}");
    let line = |mode, text: &str| ChatLine::Local {
        speaker,
        speaker_name: name.into(),
        mode,
        text: text.into(),
        position,
    };
    let common = [
        line(ChatSpeechMode::Say, &say),
        line(ChatSpeechMode::Whisper, &whisper),
        line(ChatSpeechMode::Say, &boundary),
    ];
    let speaker_lines = [
        common[0].clone(),
        common[1].clone(),
        common[2].clone(),
        line(ChatSpeechMode::Whisper, &private),
        line(ChatSpeechMode::Say, &outside),
    ];
    let listener_lines = [
        common[0].clone(),
        common[1].clone(),
        common[2].clone(),
        line(ChatSpeechMode::Whisper, "pspsps"),
    ];
    block_on(async {
        for (role, expected) in [
            ("speaker", speaker_lines.as_slice()),
            ("listener", listener_lines.as_slice()),
        ] {
            let wire = read_bound(root, &format!("s18-{role}.wire"), &binding)?;
            let (initial, after) = replay(&wire, expected).await?;
            let relog = read_bound(root, &format!("s18-{role}-relog.wire"), &binding)?;
            let (fresh, _) = replay(&relog, &[]).await?;
            assert_ne!(initial.game_session_id, fresh.game_session_id);
            assert_eq!(after, fresh.world_spatial.actor_position);
            if role == "listener" {
                assert_ne!(initial.world_spatial.actor_position, after);
            }
        }
        Ok(())
    })?
}
