//! S18 replay: the say and whisper echoes applied through `Session::apply_state_delta` (the
//! capability-7 chat-line decode in `chat.rs`) and exposed by `Session::chat_log`, the log
//! `DevClientSession` and the chat panel read.
use super::*;
use oteryn_protocol_oteryn::{decode_wire_envelope, encode_state_delta};
use std::num::NonZeroU64;
use tokio::io::DuplexStream;

const SELECTED: &[u32] = &[7, 13];

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, BoxError> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn load() -> Result<(Vec<u8>, Vec<u8>, String), BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("s18-say.bin"))?,
            std::fs::read(root.join("s18-whisper.bin"))?,
            std::fs::read_to_string(root.join("s18-expected.txt"))?,
        ));
    }
    let local = |mode, text: &str| ChatLine::Local {
        speaker: ChatSpeaker {
            identity: [0x18; 16],
            generation: NonZeroU64::MIN,
        },
        speaker_name: "Aurelia".into(),
        mode,
        text: text.into(),
        position: ActorPosition {
            x: 7,
            y: 9,
            floor: 0,
        },
    };
    let say = chat::encode_chat_line(&local(ChatSpeechMode::Say, "s18-say-line"))
        .map_err(|error| format!("{error:?}"))?;
    let whisper = chat::encode_chat_line(&local(ChatSpeechMode::Whisper, "s18-whisper-line"))
        .map_err(|error| format!("{error:?}"))?;
    Ok((
        say,
        whisper,
        format!(
            "speaker={}\ngeneration=1\nname=Aurelia\nx=7\ny=9\nfloor=0\nsay=s18-say-line\nwhisper=s18-whisper-line\n",
            "18".repeat(16)
        ),
    ))
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
fn e2e_s18_say_and_whisper_echo_replay() -> Result<(), BoxError> {
    let (say, whisper, expected) = load()?;
    let speaker = ChatSpeaker {
        identity: {
            let hex = value(&expected, "speaker")?;
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
    block_on(async {
        let (mut session, _server) = admitted().await?;
        assert_eq!(
            session.chat_log().ok_or("chat log absent")?.lines().count(),
            0
        );

        let say_line = ChatLine::Local {
            speaker,
            speaker_name: name.into(),
            mode: ChatSpeechMode::Say,
            text: value(&expected, "say")?.into(),
            position,
        };
        let event = delta(&mut session, 41, 3, &say)?;
        assert!(matches!(&event, SessionEvent::ChatLine(applied)
            if (applied.base_revision, applied.new_revision) == (3, 4) && applied.value == say_line));

        let whisper_line = ChatLine::Local {
            speaker,
            speaker_name: name.into(),
            mode: ChatSpeechMode::Whisper,
            text: value(&expected, "whisper")?.into(),
            position,
        };
        let event = delta(&mut session, 42, 4, &whisper)?;
        assert!(matches!(&event, SessionEvent::ChatLine(applied)
            if (applied.base_revision, applied.new_revision) == (4, 5) && applied.value == whisper_line));
        assert_eq!(
            session
                .chat_log()
                .ok_or("chat log absent")?
                .lines()
                .cloned()
                .collect::<Vec<_>>(),
            vec![say_line, whisper_line]
        );

        // A truncated line is refused whole: the log and the revision stay as applied.
        let before = session.chat_log().ok_or("chat log absent")?.lines().count();
        let before_revision = session.chat_revision;
        assert!(delta(&mut session, 43, 5, &whisper[..whisper.len() - 1]).is_err());
        assert_eq!(
            session.chat_log().ok_or("chat log absent")?.lines().count(),
            before
        );
        assert_eq!(session.chat_revision, before_revision);
        Ok(())
    })?
}
