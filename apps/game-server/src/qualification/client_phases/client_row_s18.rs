//! S18 client replay capture: the say and whisper echoes of public speech. The session retains
//! typed chat deltas only, so the captured lines are the server-delivered `ChatLine::Local` values
//! encoded with the production chat encoder (the same convention as C09 and S04b). The consumer is
//! `Session::apply_state_delta` and `Session::chat_log` in `crates/session`.
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatLine, ChatSpeechMode, DevClientSession};
use oteryn_protocol_oteryn::chat;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn speak(
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    let mut client = connect_world(clients, 0x0a).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let say = speak(&mut client, ChatSpeechMode::Say, "s18-say-line").await?;
    let wait_until = tokio::time::Instant::now() + Duration::from_millis(1100);
    while tokio::time::Instant::now() < wait_until {
        client.service_liveness(Duration::from_millis(20)).await?;
    }
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
            ..
        },
    ) = (&say, &whisper)
    else {
        return Err("echo is not a local line".into());
    };
    if speaker != whisper_speaker {
        return Err("say and whisper echoes carry different speakers".into());
    }
    capture(
        "s18-say.bin",
        &chat::encode_chat_line(&say).map_err(|error| format!("encode say: {error:?}"))?,
    )?;
    capture(
        "s18-whisper.bin",
        &chat::encode_chat_line(&whisper).map_err(|error| format!("encode whisper: {error:?}"))?,
    )?;
    let expected = format!(
        "speaker={}\ngeneration={}\nname={speaker_name}\nx={}\ny={}\nfloor={}\nsay=s18-say-line\nwhisper=s18-whisper-line\n",
        hex(&speaker.identity),
        speaker.generation,
        position.x,
        position.y,
        position.floor
    );
    capture("s18-expected.txt", expected.as_bytes())?;
    Ok(expected)

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=S18.client status=CAPTURED evidence={:?}",
            expected.replace('\n', ";")
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S18.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=S18.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_preserves_zero_padding_and_high_bytes() {
        assert_eq!(hex(&[]), "");
        assert_eq!(hex(&[0, 15, 128, 255]), "000f80ff");
    }
}
