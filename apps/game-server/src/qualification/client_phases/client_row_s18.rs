//! S18 live client verification and replay capture of public say and whisper echoes. The session retains
//! typed chat deltas only, so the captured lines are the server-delivered `ChatLine::Local` values
//! encoded with the production chat encoder (the same convention as C09 and S04b). The consumer is
//! `Session::apply_state_delta` and `Session::chat_log` in `crates/session`.
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatLine, ChatSpeechMode, DevClientSession};
use oteryn_protocol_oteryn::{chat, logout::LogoutResult};

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

pub(super) async fn logout_once(client: &mut DevClientSession, phase: &str) -> TestResult {
    let outcome = client.session_mut().logout().await?;
    if !accepted_logout(outcome.status, outcome.result) {
        return Err(format!("{phase}: first logout refused: {outcome:?}").into());
    }
    Ok(())
}

fn accepted_logout(status: CommandStatus, result: Option<LogoutResult>) -> bool {
    status == CommandStatus::Accepted && result == Some(LogoutResult::Accepted)
}

fn check_log(client: &DevClientSession, expected: &[ChatLine], phase: &str) -> TestResult {
    let log = client.chat_log().ok_or("S18 chat log absent")?;
    check_lines(log.lines(), expected, phase)
}

fn check_lines<'a>(
    received: impl Iterator<Item = &'a ChatLine>,
    expected: &[ChatLine],
    phase: &str,
) -> TestResult {
    let received: Vec<_> = received.collect();
    if !received.iter().copied().eq(expected) {
        return Err(format!("{phase}: expected {expected:?}, observed {received:?}").into());
    }
    Ok(())
}

async fn move_listener(
    listener: &mut DevClientSession,
    world: &BundleWorld,
    speaker: TilePos,
    goal: impl Fn(TilePos) -> bool,
) -> TestResult {
    let from = tile(listener.world_spatial().actor_position)?;
    let path = route_avoiding(world, from, &goal, |at| at != speaker)
        .ok_or("S18 listener geometry has no authored route")?;
    for direction in path {
        let from = tile(listener.world_spatial().actor_position)?;
        let expected = destination(world, from, direction).ok_or("S18 route landing absent")?;
        let outcome = listener.step(direction).await?;
        if outcome.status != CommandStatus::Accepted
            || outcome.disposition != StepDisposition::Moved
            || tile(listener.world_spatial().actor_position)? != expected
        {
            return Err(format!("S18 listener first step failed: from={from:?} direction={direction:?} expected={expected:?} outcome={outcome:?}").into());
        }
        tokio::time::sleep(Duration::from_millis(u64::from(outcome.step_wait_ms))).await;
    }
    if !goal(tile(listener.world_spatial().actor_position)?) {
        return Err("S18 listener did not reach the required geometry".into());
    }
    Ok(())
}

async fn observe_delivery(
    client: &mut DevClientSession,
    expected: &[ChatLine],
    phase: &str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    check_log(client, expected, phase)
}

async fn journey(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    root: &Path,
    head: &str,
    run: &str,
) -> TestResult<String> {
    use full_e2ef4::keep_alive;
    let mut speaker = connect_world(clients, 0x0a).await?;
    let mut listener = keep_alive(&mut speaker, connect_character(clients, 0x0b, 0)).await?;
    check_log(&speaker, &[], "speaker fresh admission")?;
    check_log(&listener, &[], "listener fresh admission")?;

    let position = speaker.world_spatial().actor_position;
    let origin = tile(position)?;
    let entities = speaker.world_entities().ok_or("S18 own entities absent")?;
    let own = entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .ok_or("S18 own actor absent")?
        .entity;
    // Provisioning fixes the Packet Two name independently of either chat log.
    let name = "Packet Two";
    let attribution = chat::ChatSpeaker {
        identity: own.identity,
        generation: std::num::NonZeroU64::new(own.generation)
            .ok_or("S18 own actor generation is zero")?,
    };
    let line = |mode, text: &str| ChatLine::Local {
        speaker: attribution,
        speaker_name: name.into(),
        mode,
        text: text.into(),
        position,
    };
    let say = format!("s18-say-{run}");
    let whisper = format!("s18-whisper-{run}");
    let boundary = format!("s18-boundary-{run}");
    let private = format!("s18-private-{run}");
    let outside = format!("s18-outside-{run}");
    let mut speaker_expected = Vec::new();
    let mut listener_expected = Vec::new();
    let mut captured = Vec::new();

    for (range, mode, text, heard) in [
        (1, ChatSpeechMode::Say, say.as_str(), Some(say.as_str())),
        (
            1,
            ChatSpeechMode::Whisper,
            whisper.as_str(),
            Some(whisper.as_str()),
        ),
        (
            2,
            ChatSpeechMode::Say,
            boundary.as_str(),
            Some(boundary.as_str()),
        ),
        (2, ChatSpeechMode::Whisper, private.as_str(), Some("pspsps")),
        (3, ChatSpeechMode::Say, outside.as_str(), None),
    ] {
        let goal = |at: TilePos| {
            let dx = at.x.abs_diff(origin.x);
            let dy = at.y.abs_diff(origin.y);
            at.floor == origin.floor
                && match range {
                    1 => dx.max(dy) == 1,
                    2 => dx <= 8 && dy <= 6 && (dx == 8 || dy == 6),
                    _ => (dx == 9 && dy <= 6) || (dy == 7 && dx <= 8),
                }
        };
        if !goal(tile(listener.world_spatial().actor_position)?) {
            keep_alive(
                &mut speaker,
                move_listener(&mut listener, world, origin, goal),
            )
            .await?;
        }
        let echo = keep_alive(&mut listener, speak(&mut speaker, mode, text)).await?;
        speaker_expected.push(line(mode, text));
        check_log(
            &speaker,
            &speaker_expected,
            "speaker echo attribution and order",
        )?;
        captured.push(echo);
        if let Some(heard) = heard {
            listener_expected.push(line(mode, heard));
        }
        let phase = format!("listener range={range} mode={mode:?}");
        keep_alive(
            &mut speaker,
            observe_delivery(&mut listener, &listener_expected, &phase),
        )
        .await?;
    }
    let received: Vec<_> = listener
        .chat_log()
        .ok_or("listener chat log absent")?
        .lines()
        .cloned()
        .collect();

    let speaker_session = speaker.join_snapshot().game_session_id;
    let listener_session = listener.join_snapshot().game_session_id;
    keep_alive(&mut listener, logout_once(&mut speaker, "speaker")).await?;
    logout_once(&mut listener, "listener").await?;
    drop((speaker, listener));
    for (index, previous) in [(1, speaker_session), (0, listener_session)] {
        let mut fresh = connect_character(clients, 0x0c, index).await?;
        if fresh.join_snapshot().game_session_id == previous {
            return Err("S18 relog reused the old gameplay session".into());
        }
        observe_delivery(
            &mut fresh,
            &[],
            "fresh relog must not show historical speech",
        )
        .await?;
        logout_once(&mut fresh, "relogged character").await?;
    }

    let binding = format!("{head} {run}\n");
    for (role, lines) in [("speaker", &captured), ("listener", &received)] {
        for (index, payload) in lines.iter().enumerate() {
            let mut bytes = binding.as_bytes().to_vec();
            bytes.extend(
                chat::encode_chat_line(payload)
                    .map_err(|error| format!("S18 encode: {error:?}"))?,
            );
            capture(root, &format!("s18-{role}-{index}.bin"), &bytes)?;
        }
    }
    let expected = format!(
        "speaker={}\ngeneration={}\nname={name}\nx={}\ny={}\nfloor={}\n",
        hex(&attribution.identity),
        attribution.generation,
        position.x,
        position.y,
        position.floor,
    );
    capture(
        root,
        "s18-expected.txt",
        format!("{binding}{expected}").as_bytes(),
    )?;
    Ok(format!(
        "head={head};run={run};speaker=Packet_Two;near=verified;boundary=verified;outside=verified;relog=verified"
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let started = tokio::time::Instant::now();
    let result = verdict::indexed_timeout(
        Duration::from_secs(90),
        verdict::observe(async {
            let root = required("E2E_ALL_CAPTURE_DIR")?;
            let head = required("S18_CAPTURE_HEAD")?;
            let run = required("S18_CAPTURE_RUN")?;
            if head.len() != 40
                || run.len() != 32
                || !head
                    .bytes()
                    .chain(run.bytes())
                    .all(|b| b.is_ascii_hexdigit())
            {
                return Err("S18 capture requires a Git head and unique run token".into());
            }
            journey(clients, world, Path::new(&root), &head, &run).await
        }),
    )
    .await;
    let seconds = started.elapsed().as_secs_f64();
    match result {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=S18.client status=CAPTURED seconds={seconds:.3} evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S18.client status=FAIL seconds={seconds:.3} evidence={error}"
        )),
        Err(_) => evidence(&format!(
            "e2e_all scenario=S18.client status=FAIL seconds={seconds:.3} evidence=timeout"
        )),
    }
}

#[cfg(test)]
mod audit_tests {
    use super::*;

    #[test]
    fn rejects_wrong_attribution_delivery_and_relogged_history() {
        let line = |identity, generation, name: &str, text: &str| ChatLine::Local {
            speaker: chat::ChatSpeaker {
                identity: [identity; 16],
                generation: std::num::NonZeroU64::new(generation).expect("nonzero audit fixture"),
            },
            speaker_name: name.into(),
            mode: ChatSpeechMode::Whisper,
            text: text.into(),
            position: ActorPosition {
                x: 32369,
                y: 32238,
                floor: 7,
            },
        };
        let expected = [line(1, 2, "Packet Two", "pspsps")];
        assert!(check_lines(expected.iter(), &expected, "control").is_ok());
        for invalid in [
            line(2, 2, "Packet Two", "pspsps"),
            line(1, 3, "Packet Two", "pspsps"),
            line(1, 2, "Packet One", "pspsps"),
            line(1, 2, "Packet Two", "private text"),
        ] {
            assert!(check_lines([&invalid].into_iter(), &expected, "mutation").is_err());
        }
        assert!(check_lines(std::iter::empty(), &expected, "missing listener echo").is_err());
        assert!(check_lines(expected.iter(), &[], "outside or relog leakage").is_err());
    }

    #[test]
    fn rejects_busy_as_the_first_logout_outcome() {
        assert!(!accepted_logout(
            CommandStatus::Accepted,
            Some(LogoutResult::Busy)
        ));
        assert!(accepted_logout(
            CommandStatus::Accepted,
            Some(LogoutResult::Accepted)
        ));
    }
}
