//! C01 records the server stream and replays it through the native Session.
use super::*;
use oteryn_dev_client::native_session::{Admission, Session, SessionStream};
use oteryn_dev_client::{CLIENT_SUPPORTED_CAPABILITIES, SessionEvent};
use oteryn_protocol_oteryn::attack::{
    self, ActorCombatState, AttackIntentDisposition, ChaseMode, FightModes,
};
use std::io::{self, Write};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const MAX_CAPTURE_BYTES: usize = 8 * 1024 * 1024;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(20), verdict::observe(capture(clients))).await;
    match result {
        Ok(Ok(())) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=C01.client status=PASS evidence=exact-admission-and-combat-stream-replay",
        ),
        Ok(Err(error)) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C01.client status=FAIL evidence={error}"
        )),
        Err(_) => {
            evidence("SEAM_EVIDENCE e2e_all scenario=C01.client status=FAIL evidence=timeout")
        }
    }
}

// Allocate atomically with owner-only permissions, including under the bare node-boot wrapper.
fn fresh_capture_directory(parent: Option<&Path>) -> TestResult<PathBuf> {
    let parent = match parent {
        Some(parent) => parent.canonicalize()?,
        None => std::env::temp_dir(),
    };
    let output = std::process::Command::new("mktemp")
        .arg("-d")
        .arg(parent.join("q-c01-wire.XXXXXX"))
        .output()?;
    if !output.status.success() {
        return Err("C01 could not allocate a fresh capture directory".into());
    }
    Ok(PathBuf::from(
        std::str::from_utf8(&output.stdout)?.trim_end(),
    ))
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let parent = std::env::var_os("E2E_ALL_CAPTURE_DIR").map(PathBuf::from);
    let directory = fresh_capture_directory(parent.as_deref())?;
    let path = directory.join("c01-server-stream.bin");
    evidence(&format!("C01_CAPTURE directory={}", directory.display()));
    let index = 1;
    wait_character_released(clients, index).await?;
    let token = if requires_platform_grant(&clients.accounts[index]) {
        platform_row_grant(clients, index).await?
    } else {
        let generation = platform_generation(clients.descriptor, &clients.accounts[index]).await?;
        let (nonce, attempt) = fresh_grant_material(0xc1)?;
        let grant = Grant {
            signing: clients.signing,
            key_id: clients.key_id,
            account_id: &clients.accounts[index],
            character_id: clients.characters[index],
            world_id: clients.world,
            channel_id: clients.channel,
            security_generation: generation,
            scope_generation: clients.scope_generation,
            nonce,
            attempt,
        };
        sign_grant(&grant, now_seconds()?)
    };
    let tcp = tokio::net::TcpStream::connect(clients.address).await?;
    let tls = connector(clients.certificate, &EXACT)?
        .connect(ServerName::try_from("localhost")?, tcp)
        .await?;
    if tls.get_ref().1.alpn_protocol() != EXACT.alpn {
        return Err("C01 gameplay ALPN absent".into());
    }
    let admission = Admission {
        appearance_items: None,
        schema_revision: 1,
        character_id: crate::foundation::CharacterId::decode(&clients.characters[index])?,
        admission_material: token.as_bytes(),
        client_build_id: "oteryn-dev-client/world-qualification",
        supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
        deadline: Duration::from_secs(20),
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let (live, mut session) = observe(
        RecordingStream {
            stream: tls,
            bytes: Arc::clone(&bytes),
        },
        admission,
    )
    .await?;
    file.write_all(&bytes.lock().map_err(|_| "C01 capture lock poisoned")?)?;
    drop(file);

    let replay = replay_live_combat_stream(&path, admission).await?;
    if replay != live {
        return Err(
            "C01 replay session, admission bytes or combat sequence differ from live".into(),
        );
    }
    evidence(&format!(
        "C01_REPLAY test=replay_live_combat_stream session={:?} result_sequence={} delta_sequence={}",
        live.session, live.result_sequence, live.delta_sequence
    ));
    let logout = session.logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C01 clean logout refused: {logout:?}").into());
    }
    let mut fresh = connect_world(clients, 0xc2).await?;
    if fresh.join_snapshot().game_session_id == live.session {
        return Err("C01 relog reused the live session".into());
    }
    check_idle_baseline(fresh.session_mut())?;
    evidence("C01_RELOG fresh_session=true chase=stand target=none in_fight=false");
    Ok(())
}

#[derive(PartialEq, Eq)]
struct Observation {
    session: oteryn_protocol_oteryn::GameSessionId,
    snapshot: Vec<u8>,
    result_sequence: u64,
    delta_sequence: u64,
}

fn check_idle_baseline(stream: &Session<impl SessionStream>) -> TestResult<()> {
    let initial = idle_baseline();
    if stream.combat_state() != Some(&initial) {
        return Err("C01 admission did not install the idle combat baseline".into());
    }
    Ok(())
}

fn idle_baseline() -> ActorCombatState {
    ActorCombatState {
        target: None,
        modes: FightModes::DEFAULT,
        in_fight: false,
    }
}

async fn observe<S: SessionStream>(
    stream: S,
    admission: Admission<'_>,
) -> TestResult<(Observation, Session<S>)> {
    let mut session = Session::admit(stream, admission).await?;
    let snapshot = session
        .admission_snapshot(attack::STATE_DOMAIN_ACTOR_COMBAT_STATE)
        .ok_or("domain-10 combat admission snapshot absent")?;
    if snapshot.snapshot_type != attack::SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1 {
        return Err("C01 unexpected combat admission snapshot type".into());
    }
    check_idle_baseline(&session)?;
    let revision = snapshot.revision;
    let snapshot = snapshot.payload.clone();
    let expected = ActorCombatState {
        modes: FightModes {
            chase: ChaseMode::Chase,
            ..FightModes::DEFAULT
        },
        ..idle_baseline()
    };
    let reply = session.set_fight_modes(&expected.modes).await?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("fight/chase command refused: {reply:?}").into());
    }

    let delta_sequence = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let events = session.take_events();
            if let Some(sequence) = verify_combat_batch(
                events,
                session.combat_state(),
                reply.result_server_sequence,
                revision,
                expected,
            )? {
                return Ok::<_, Box<dyn std::error::Error>>(sequence);
            }
            session.service_liveness(Duration::from_millis(20)).await?;
        }
    })
    .await
    .map_err(|_| "fight/chase domain-10 delta absent")??;
    Ok((
        Observation {
            session: session.join_snapshot().game_session_id,
            snapshot,
            result_sequence: reply.result_server_sequence,
            delta_sequence,
        },
        session,
    ))
}

fn verify_combat_batch(
    events: impl IntoIterator<Item = SessionEvent>,
    installed: Option<&ActorCombatState>,
    result_sequence: u64,
    admission_revision: u64,
    expected: ActorCombatState,
) -> TestResult<Option<u64>> {
    let mut first = None;
    for event in events {
        if let SessionEvent::ActorCombatState(delta) = event
            && delta.server_sequence > result_sequence
        {
            if delta.value != expected
                || (first.is_none() && delta.base_revision != admission_revision)
            {
                return Err(format!("C01 conflicting post-result combat delta: {delta:?}").into());
            }
            first.get_or_insert(delta.server_sequence);
        }
    }
    if first.is_some() && installed != Some(&expected) {
        return Err("C01 installed combat state differs from expected".into());
    }
    Ok(first)
}

async fn replay_live_combat_stream(
    path: &Path,
    admission: Admission<'_>,
) -> TestResult<Observation> {
    let bytes = std::fs::read(path)?;
    if bytes.is_empty() || bytes.len() > MAX_CAPTURE_BYTES {
        return Err("C01 replay transcript outside capture bounds".into());
    }
    let (client, mut peer) = tokio::io::duplex(bytes.len());
    peer.write_all(&bytes).await?;
    // Keep the peer open while the session reads the exact recorded server frames.
    // No bootstrap materials are written to the transcript.
    let result = observe(client, admission)
        .await
        .map(|(observation, _)| observation);
    drop(peer);
    result
}

struct RecordingStream<S> {
    stream: S,
    bytes: Arc<Mutex<Vec<u8>>>,
}

impl<S: AsyncRead + Unpin> AsyncRead for RecordingStream<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let start = buffer.filled().len();
        match Pin::new(&mut this.stream).poll_read(cx, buffer) {
            Poll::Ready(Ok(())) => {
                let mut bytes = this
                    .bytes
                    .lock()
                    .map_err(|_| io::Error::other("C01 capture lock poisoned"))?;
                let read = &buffer.filled()[start..];
                if read.len() > MAX_CAPTURE_BYTES - bytes.len() {
                    return Poll::Ready(Err(io::Error::other("C01 capture limit exceeded")));
                }
                bytes.extend_from_slice(read);
                Poll::Ready(Ok(()))
            }
            outcome => outcome,
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for RecordingStream<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().stream).poll_write(cx, bytes)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_capture_parent_creates_two_independent_directories() -> TestResult {
        let first = fresh_capture_directory(None)?;
        let second = fresh_capture_directory(None)?;
        assert_ne!(first, second);
        std::fs::write(first.join("c01-server-stream.bin"), b"prior capture")?;
        assert!(!second.join("c01-server-stream.bin").exists());
        std::fs::remove_dir_all(first)?;
        std::fs::remove_dir(second)?;
        Ok(())
    }

    #[test]
    fn rejects_later_conflict_and_wrong_installed_state() -> TestResult {
        use oteryn_dev_client::native_session::AppliedDelta;

        let initial = idle_baseline();
        let mut expected = initial;
        expected.modes.chase = ChaseMode::Chase;
        let event = |server_sequence, base_revision, value| {
            SessionEvent::ActorCombatState(AppliedDelta {
                server_sequence,
                base_revision,
                new_revision: base_revision + 1,
                value,
            })
        };
        assert_eq!(
            verify_combat_batch([event(3, 1, expected)], Some(&expected), 1, 1, expected)?,
            Some(3)
        );
        assert!(
            verify_combat_batch(
                [event(3, 1, expected), event(4, 2, initial)],
                Some(&initial),
                1,
                1,
                expected,
            )
            .is_err()
        );
        assert!(
            verify_combat_batch(
                [
                    event(3, 1, expected),
                    event(4, 2, initial),
                    event(5, 3, expected)
                ],
                Some(&expected),
                1,
                1,
                expected,
            )
            .is_err()
        );
        assert!(
            verify_combat_batch([event(3, 1, expected)], Some(&initial), 1, 1, expected).is_err()
        );
        Ok(())
    }

    #[tokio::test]
    async fn recording_preserves_inbound_fragments_without_client_bytes() -> TestResult {
        let (stream, mut peer) = tokio::io::duplex(64);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let mut recording = RecordingStream {
            stream,
            bytes: Arc::clone(&bytes),
        };
        recording.write_all(b"client request").await?;
        peer.write_all(b"server frames").await?;
        let mut first = [0; 3];
        let mut rest = [0; 10];
        recording.read_exact(&mut first).await?;
        recording.read_exact(&mut rest).await?;
        assert_eq!(&first, b"ser");
        assert_eq!(&rest, b"ver frames");
        assert_eq!(
            &*bytes.lock().map_err(|_| "C01 capture lock poisoned")?,
            b"server frames"
        );
        Ok(())
    }
}
