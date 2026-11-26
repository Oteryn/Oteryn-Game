//! S18 live client verification and replay capture of public say and whisper echoes. The session retains
//! typed chat deltas only, so the captured lines are the server-delivered `ChatLine::Local` values
//! encoded with the production chat encoder (the same convention as C09 and S04b). The consumer is
//! `Session::apply_state_delta` and `Session::chat_log` in `crates/session`.
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatLine, ChatSpeechMode, DevClientSession};
use oteryn_protocol_oteryn::chat;

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write as _;

    std::fs::File::create_new(root.join(name))
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|error| format!("S18 capture write failed for {name}: {error}"))?;
    Ok(())
}

pub(super) async fn speak(
    client: &mut DevClientSession,
    mode: ChatSpeechMode,
    text: &str,
) -> TestResult<ChatLine> {
    let outcome = client
        .chat(&ChatIntent::Say {
            mode,
            text: text.into(),
        })
        .await?;
    if outcome.status != CommandStatus::Accepted || outcome.disposition != ChatDisposition::Ok {
        return Err(format!("speech {mode:?} refused: {outcome:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let found = client.chat_log().and_then(|log| {
            log.lines()
                .find(|line| {
                    matches!(line, ChatLine::Local { mode: seen, text: said, .. }
                        if *seen == mode && said == text)
                })
                .cloned()
        });
        if let Some(line) = found {
            return Ok(line);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("speech {mode:?} echo absent from the client chat log").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn journey(clients: &SeamClients<'_>, capture_root: Option<&Path>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x0a).await?;
    let own_position = client.world_spatial().actor_position;
    if !client
        .chat_log()
        .ok_or("chat log absent at admission")?
        .is_empty()
    {
        return Err("fresh admission already has chat lines".into());
    }
    let say = speak(&mut client, ChatSpeechMode::Say, "s18-say-line").await?;
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let whisper = speak(&mut client, ChatSpeechMode::Whisper, "s18-whisper-line").await?;
    let (
        ChatLine::Local {
            speaker,
            speaker_name,
            position,
            ..
        },
        ChatLine::Local {
            speaker: whisper_speaker,
            speaker_name: whisper_name,
            position: whisper_position,
            ..
        },
    ) = (&say, &whisper)
    else {
        return Err("echo is not a local line".into());
    };
    if speaker != whisper_speaker {
        return Err("say and whisper echoes carry different speakers".into());
    }
    if speaker_name.is_empty() || speaker_name != whisper_name {
        return Err("say and whisper echoes lack a consistent speaker name".into());
    }
    if *position != own_position || *whisper_position != own_position {
        return Err("say and whisper echo positions must match the speaking client".into());
    }
    if !client
        .chat_log()
        .ok_or("chat log absent after speech")?
        .lines()
        .eq([&say, &whisper])
    {
        return Err("chat log must contain exactly the say and whisper echoes in order".into());
    }
    let logout = client.session_mut().logout().await?;
    if logout.status != CommandStatus::Accepted
        || logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("speech session logout refused: {logout:?}").into());
    }
    let expected = format!(
        "speaker={}\ngeneration={}\nname={speaker_name}\nx={}\ny={}\nfloor={}\nsay=s18-say-line\nwhisper=s18-whisper-line\n",
        hex(&speaker.identity),
        speaker.generation,
        position.x,
        position.y,
        position.floor
    );
    if let Some(root) = capture_root {
        capture(
            root,
            "s18-say.bin",
            &chat::encode_chat_line(&say).map_err(|error| format!("encode say: {error:?}"))?,
        )?;
        capture(
            root,
            "s18-whisper.bin",
            &chat::encode_chat_line(&whisper)
                .map_err(|error| format!("encode whisper: {error:?}"))?,
        )?;
        capture(root, "s18-expected.txt", expected.as_bytes())?;
    }
    Ok(expected)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let capture_dir = std::env::var_os("E2E_ALL_CAPTURE_DIR");
    let capture_root = capture_dir.as_deref().map(Path::new);
    let started = tokio::time::Instant::now();
    let result = verdict::indexed_timeout(
        Duration::from_secs(60),
        verdict::observe(journey(clients, capture_root)),
    )
    .await;
    let seconds = started.elapsed().as_secs_f64();
    match result {
        Ok(Ok(expected)) => {
            let status = if capture_root.is_some() {
                "CAPTURED"
            } else {
                "PASS"
            };
            evidence(&format!(
                "e2e_all scenario=S18.client status={status} seconds={seconds:.3} evidence={:?}",
                expected.replace('\n', ";")
            ));
        }
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S18.client status=FAIL seconds={seconds:.3} evidence={error}"
        )),
        Err(_) => evidence(&format!(
            "e2e_all scenario=S18.client status=FAIL seconds={seconds:.3} evidence=timeout"
        )),
    }
}
