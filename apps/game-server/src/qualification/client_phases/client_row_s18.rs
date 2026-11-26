//! S18 verifies attributed public speech on two live clients and captures their original
//! server envelopes, including fresh admissions, for the Session replay.
use super::*;
use oteryn_dev_client::native_session::{Admission, Session};
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatLine, ChatSpeechMode, DevClientSession};
use oteryn_protocol_oteryn::{chat, logout::LogoutResult};
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

type WireBytes = Rc<RefCell<Vec<u8>>>;
type RecordedSession = Session<RecordingStream<tokio_rustls::client::TlsStream<TcpStream>>>;

struct RecordingStream<S> {
    inner: S,
    received: WireBytes,
}

impl<S: AsyncRead + Unpin> AsyncRead for RecordingStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buffer.filled().len();
        let result = std::task::ready!(Pin::new(&mut self.inner).poll_read(cx, buffer));
        if result.is_ok() {
            let received = &buffer.filled()[before..];
            let mut bytes = self.received.borrow_mut();
            if bytes.len() + received.len() > 8 * 1024 * 1024 {
                return Poll::Ready(Err(io::Error::other("S18 wire capture exceeds 8 MiB")));
            }
            bytes.extend_from_slice(received);
        }
        Poll::Ready(result)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for RecordingStream<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, bytes)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

async fn recorded_character(
    clients: &SeamClients<'_>,
    index: usize,
) -> TestResult<(RecordedSession, WireBytes)> {
    connect_character_using(clients, 0x0b, index, async |request| {
        let received = WireBytes::default();
        let stream = tokio::time::timeout(request.deadline, async {
            let tcp = TcpStream::connect(request.address).await?;
            let stream = connector(request.root_certificate, &EXACT)?
                .connect(ServerName::try_from("localhost")?, tcp)
                .await?;
            if stream.get_ref().1.alpn_protocol() != EXACT.alpn {
                return Err("S18 negotiated wrong ALPN".into());
            }
            Ok::<_, Box<dyn std::error::Error>>(stream)
        })
        .await??;
        let session = Session::admit(
            RecordingStream {
                inner: stream,
                received: Rc::clone(&received),
            },
            Admission {
                appearance_items: None,
                schema_revision: request.schema_revision,
                character_id: request.character_id,
                admission_material: request.admission_material,
                client_build_id: request.client_build_id,
                supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
                deadline: request.deadline,
            },
        )
        .await?;
        Ok((session, received))
    })
    .await
}

async fn service_while<T>(
    parked: &mut RecordedSession,
    action: impl Future<Output = TestResult<T>>,
) -> TestResult<T> {
    let (done, mut stop) = tokio::sync::oneshot::channel();
    let (result, serviced) = tokio::join!(
        async {
            let result = action.await;
            let _ = done.send(());
            result
        },
        async {
            while stop
                .try_recv()
                .is_err_and(|error| error == tokio::sync::oneshot::error::TryRecvError::Empty)
            {
                parked
                    .service_liveness_discarding_events(Duration::from_millis(100))
                    .await?;
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        },
    );
    serviced?;
    result
}

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

type CommandReceipt = (u64, u64);

async fn logout_recorded(client: &mut RecordedSession, phase: &str) -> TestResult<CommandReceipt> {
    let outcome = client.logout().await?;
    require_mechanic_acceptance(outcome.status, phase)
        .map_err(|error| format!("{error}: {outcome:?}"))?;
    if outcome.result != Some(LogoutResult::Accepted) {
        return Err(format!("{phase}: first logout refused: {outcome:?}").into());
    }
    Ok((outcome.command_id, outcome.result_server_sequence))
}

fn check_log(client: &RecordedSession, expected: &[ChatLine], phase: &str) -> TestResult {
    let log = client.chat_log().ok_or("S18 chat log absent")?;
    check_lines(log.lines(), expected, phase)
}

fn check_lines<'a>(
    received: impl Iterator<Item = &'a ChatLine>,
    expected: &[ChatLine],
    phase: &str,
) -> TestResult {
    let mut received = received;
    for (index, expected) in expected.iter().enumerate() {
        let actual = received.next();
        if actual != Some(expected) {
            return Err(format!(
                "{phase}: line {index} expected {expected:?}, observed {actual:?}"
            )
            .into());
        }
    }
    if let Some(extra) = received.next() {
        return Err(format!("{phase}: unexpected extra line {extra:?}").into());
    }
    Ok(())
}

async fn move_listener(
    listener: &mut RecordedSession,
    world: &BundleWorld,
    speaker: TilePos,
    goal: impl Fn(TilePos) -> bool,
    receipts: &mut Vec<CommandReceipt>,
) -> TestResult {
    let from = tile(listener.world_spatial().actor_position)?;
    let path = route_avoiding(world, from, &goal, |at| at != speaker)
        .ok_or("S18 listener geometry has no authored route")?;
    for direction in path {
        let from = tile(listener.world_spatial().actor_position)?;
        let expected = destination(world, from, direction).ok_or("S18 route landing absent")?;
        let outcome = listener.step(direction).await?;
        require_mechanic_acceptance(outcome.status, "S18 listener step")
            .map_err(|error| format!("{error}: {outcome:?}"))?;
        if outcome.disposition != StepDisposition::Moved
            || tile(listener.world_spatial().actor_position)? != expected
        {
            return Err(format!("S18 listener first step failed: from={from:?} direction={direction:?} expected={expected:?} outcome={outcome:?}").into());
        }
        receipts.push((outcome.command_id, outcome.result_server_sequence));
        tokio::time::sleep(Duration::from_millis(u64::from(outcome.step_wait_ms))).await;
    }
    if !goal(tile(listener.world_spatial().actor_position)?) {
        return Err("S18 listener did not reach the required geometry".into());
    }
    Ok(())
}

async fn observe_delivery(
    client: &mut RecordedSession,
    expected: &[ChatLine],
    phase: &str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
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
    let (mut speaker, speaker_wire) = recorded_character(clients, 1).await?;
    let (mut listener, listener_wire) =
        service_while(&mut speaker, recorded_character(clients, 0)).await?;
    let listener_start = listener.world_spatial().actor_position;
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
    let outside_whisper = format!("s18-outside-whisper-{run}");
    let mut speaker_receipts = Vec::new();
    let mut listener_receipts = Vec::new();
    let mut speaker_expected = Vec::new();
    let mut listener_expected = Vec::new();

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
        (3, ChatSpeechMode::Whisper, outside_whisper.as_str(), None),
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
            service_while(
                &mut speaker,
                move_listener(&mut listener, world, origin, goal, &mut listener_receipts),
            )
            .await?;
        }
        let outcome = service_while(&mut listener, async {
            Ok(speaker
                .chat(&ChatIntent::Say {
                    mode,
                    text: text.into(),
                })
                .await?)
        })
        .await?;
        require_mechanic_acceptance(outcome.status, "S18 speech")
            .map_err(|error| format!("{error}: {outcome:?}"))?;
        if outcome.disposition != ChatDisposition::Ok {
            return Err(format!("speech {mode:?} first command refused: {outcome:?}").into());
        }
        speaker_receipts.push((outcome.command_id, outcome.result_server_sequence));
        speaker_expected.push(line(mode, text));
        if let Some(heard) = heard {
            listener_expected.push(line(mode, heard));
        }
        let listener_at = listener.world_spatial().actor_position;
        let phase = format!("listener range={range} mode={mode:?} position={listener_at:?}");
        let (own_result, listener_result) = tokio::join!(
            observe_delivery(
                &mut speaker,
                &speaker_expected,
                "speaker echo attribution and order"
            ),
            observe_delivery(&mut listener, &listener_expected, &phase),
        );
        own_result?;
        if let Err(error) = listener_result {
            capture(
                root,
                "s18-diagnostic-listener.wire",
                &listener_wire.borrow(),
            )?;
            return Err(error);
        }
    }
    let listener_landing = listener.world_spatial().actor_position;
    if listener_start == listener_landing {
        return Err("S18 listener travel did not change its position".into());
    }

    let speaker_session = speaker.join_snapshot().game_session_id;
    let listener_session = listener.join_snapshot().game_session_id;
    speaker_receipts
        .push(service_while(&mut listener, logout_recorded(&mut speaker, "speaker")).await?);
    listener_receipts.push(logout_recorded(&mut listener, "listener").await?);
    drop((speaker, listener));

    let binding = format!("{head} {run}\n");
    let save = |role: &str,
                wire: &WireBytes,
                session: [u8; 16],
                receipts: &[CommandReceipt]|
     -> TestResult {
        let mut bytes = binding.as_bytes().to_vec();
        let mut provenance = hex(&session);
        for (command, sequence) in receipts {
            use std::fmt::Write as _;
            write!(provenance, " {command}:{sequence}")?;
        }
        bytes.extend_from_slice(provenance.as_bytes());
        bytes.push(b'\n');
        bytes.extend_from_slice(&wire.borrow());
        capture(root, &format!("s18-{role}.wire"), &bytes)
    };
    save(
        "speaker",
        &speaker_wire,
        *speaker_session.as_bytes(),
        &speaker_receipts,
    )?;
    save(
        "listener",
        &listener_wire,
        *listener_session.as_bytes(),
        &listener_receipts,
    )?;
    for (index, previous, expected_position, role) in [
        (1, speaker_session, position, "speaker-relog"),
        (0, listener_session, listener_landing, "listener-relog"),
    ] {
        let (mut fresh, wire) = recorded_character(clients, index).await?;
        if fresh.join_snapshot().game_session_id == previous {
            return Err("S18 relog reused the old gameplay session".into());
        }
        if fresh.world_spatial().actor_position != expected_position {
            return Err(format!(
                "S18 {role} persisted position differs: expected={expected_position:?} actual={:?}",
                fresh.world_spatial().actor_position
            )
            .into());
        }
        observe_delivery(
            &mut fresh,
            &[],
            "fresh relog must not show historical speech",
        )
        .await?;
        let session = *fresh.join_snapshot().game_session_id.as_bytes();
        let receipt = logout_recorded(&mut fresh, "relogged character").await?;
        save(role, &wire, session, &[receipt])?;
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
            let checkout = std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()?;
            if !checkout.status.success() {
                return Err("S18 capture head unavailable".into());
            }
            let head = String::from_utf8(checkout.stdout)?;
            let root = Path::new(&root);
            let run = root
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .filter(|name| name.starts_with("captures.") && name.len() == 15)
                .ok_or("S18 requires the cloud wrapper's fresh capture directory")?;
            journey(clients, world, root, head.trim(), run).await
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
}
