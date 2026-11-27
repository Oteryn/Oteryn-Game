//! Replay the original server frames through Session's typed achievement reader.
use super::*;
use oteryn_dev_client::native_session::{Admission, Session, SessionStream};
use oteryn_protocol_oteryn::account_achievements as wire;
use std::io::{self, Cursor};
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};

const MAX_CAPTURE_BYTES: usize = 4 * 1024 * 1024;

struct Recording<'a, S> {
    stream: S,
    bytes: &'a Mutex<Vec<u8>>,
}

impl<S: AsyncRead + Unpin> AsyncRead for Recording<'_, S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let offset = buffer.filled().len();
        match Pin::new(&mut self.stream).poll_read(cx, buffer) {
            Poll::Ready(Ok(())) => {
                let mut bytes = self
                    .bytes
                    .lock()
                    .map_err(|_| io::Error::other("P11 capture lock poisoned"))?;
                let received = &buffer.filled()[offset..];
                if bytes.len() + received.len() > MAX_CAPTURE_BYTES {
                    return Poll::Ready(Err(io::Error::other("P11 capture exceeded byte limit")));
                }
                bytes.extend_from_slice(received);
                Poll::Ready(Ok(()))
            }
            result => result,
        }
    }
}

impl<S: tokio::io::AsyncWrite + Unpin> tokio::io::AsyncWrite for Recording<'_, S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(cx, buffer)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Observation {
    session: [u8; 16],
    commands: [u64; 3],
    sequences: [u64; 3],
    page: wire::AccountAchievementsResult,
}

async fn observe<S: SessionStream>(client: &mut Session<S>) -> TestResult<Observation> {
    use oteryn_dev_client::client_runtime::account_achievements::Response;
    let session = *client.join_snapshot().game_session_id.as_bytes();
    let first = client.next_command_id();
    let before = client.last_server_sequence();
    let Response::Page(page) = client.account_achievements(0).await? else {
        return Err("page query observed=refused".into());
    };
    let sequence = client.last_server_sequence();
    let next = client.next_command_id();
    if sequence <= before || first.checked_add(1) != Some(next) {
        return Err("first achievement response did not advance continuity".into());
    }
    if wire::decode_account_achievements_query(&[0x08])
        != Err(wire::AccountAchievementsError::Malformed)
        || client.next_command_id() != next
        || client.last_server_sequence() != sequence
    {
        return Err("malformed query was not refused locally without submission".into());
    }
    if client.account_achievements(0).await? != Response::Page(page.clone()) {
        return Err("local malformed refusal changed the page".into());
    }
    let reread = client.last_server_sequence();
    if reread <= sequence || next.checked_add(1) != Some(client.next_command_id()) {
        return Err("achievement reread did not advance continuity".into());
    }
    let pagination = client.next_command_id();
    let beyond = page
        .fact_count
        .div_ceil(wire::ACCOUNT_ACHIEVEMENTS_PAGE_ROWS as u32)
        .max(1);
    let expected = wire::AccountAchievementsResult {
        page: beyond,
        has_more: false,
        rows: Vec::new(),
        total_points: page.total_points,
        fact_count: page.fact_count,
    };
    if expected == page || client.account_achievements(beyond).await? != Response::Page(expected) {
        return Err("P11 pagination did not transition to the exact requested page".into());
    }
    let paginated = client.last_server_sequence();
    if paginated <= reread || pagination.checked_add(1) != Some(client.next_command_id()) {
        return Err("achievement pagination did not advance continuity".into());
    }
    if client.logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("achievement reader logout was refused".into());
    }
    for event in client.take_events() {
        full_row_p11::check_unexpected_notice(&event)?;
    }
    Ok(Observation {
        session,
        commands: [first, next, pagination],
        sequences: [sequence, reread, paginated],
        page,
    })
}

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CapturedCommandResult {
    command_id: u64,
    game_session_id: String,
    payload: Vec<u8>,
    server_sequence: u64,
    status: u32,
}

fn command_envelopes(
    transcript: &[u8],
    session: [u8; 16],
) -> TestResult<Vec<CapturedCommandResult>> {
    use oteryn_protocol_oteryn::{MessageType, decode_command_result};
    if session[6] >> 4 != 7 || session[8] & 0xc0 != 0x80 {
        return Err("P11 admitted session is not UUIDv7".into());
    }
    let game_session_id = uuid_text(&session);
    let Reply::Frames(records) = frames(transcript) else {
        return Err("P11 capture has no server frames".into());
    };
    let mut envelopes = Vec::new();
    for record in records {
        let frame = decode_wire_envelope(&record)?;
        if frame.message_type() == MessageType::CommandResult {
            let result = decode_command_result(frame.payload())?;
            envelopes.push(CapturedCommandResult {
                command_id: result.command_id,
                game_session_id: game_session_id.clone(),
                payload: result.payload.to_vec(),
                server_sequence: frame.server_sequence(),
                status: result.status as u32,
            });
        }
    }
    Ok(envelopes)
}

fn provenance(
    transcript: &[u8],
    live: &Observation,
    character: &[u8; 16],
    envelopes: &[CapturedCommandResult],
    malformed: &[u8],
) -> TestResult<String> {
    use sha2::{Digest, Sha256};
    let results: Vec<_> = envelopes
        .iter()
        .map(|record| (record.command_id, record.server_sequence))
        .collect();
    if results.len() != 4
        || !results.iter().take(3).map(|&(id, _)| id).eq(live.commands)
        || live.commands[2].checked_add(1) != Some(results[3].0)
        || results.windows(2).any(|pair| pair[0].1 >= pair[1].1)
    {
        return Err("P11 capture command results do not match the four live commands".into());
    }
    Ok(serde_json::json!({
        "frames_sha256": Sha256::digest(transcript).as_slice(),
        "malformed_sha256": Sha256::digest(malformed).as_slice(),
        "character": character,
        "session": uuid_text(&live.session),
        "results": results,
        "observation_sequences": live.sequences,
        "page": format!("{:?}", live.page),
    })
    .to_string())
}

#[test]
fn p11_capture_envelope_requires_the_exact_five_fields() -> TestResult {
    let envelope = CapturedCommandResult {
        command_id: 2,
        game_session_id: "01934f10-7c00-7000-8000-000000000004".into(),
        payload: vec![8, 1],
        server_sequence: 3,
        status: 1,
    };
    let value = serde_json::to_value(&envelope)?;
    let object = value.as_object().ok_or("P11 envelope is not an object")?;
    assert_eq!(object.len(), 5);
    assert_eq!(
        serde_json::from_value::<CapturedCommandResult>(value.clone())?,
        envelope
    );
    for field in [
        "command_id",
        "game_session_id",
        "payload",
        "server_sequence",
        "status",
    ] {
        let mut missing = object.clone();
        assert!(missing.remove(field).is_some());
        assert!(serde_json::from_value::<CapturedCommandResult>(missing.into()).is_err());
    }
    let mut extra = object.clone();
    extra.insert("extra".into(), serde_json::Value::Null);
    assert!(serde_json::from_value::<CapturedCommandResult>(extra.into()).is_err());
    Ok(())
}

#[test]
fn e2e_p11_original_frame_replay() -> TestResult {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        assert!(std::env::var_os("OTERYN_P11_LIVE_PROVENANCE").is_none());
        return Ok(());
    };
    let expected = std::env::var("OTERYN_P11_LIVE_PROVENANCE")?;
    let directory = Path::new(&directory);
    if std::fs::read_to_string(directory.join("p11-provenance.json"))? != expected {
        return Err("P11 capture provenance differs from this run's live verdict".into());
    }
    let metadata: serde_json::Value = serde_json::from_str(&expected)?;
    let character: [u8; 16] = serde_json::from_value(metadata["character"].clone())?;
    let transcript = std::fs::read(directory.join("p11-frames.bin"))?;
    let malformed = std::fs::read(directory.join("p11-malformed-frames.bin"))?;
    let envelopes: Vec<CapturedCommandResult> =
        serde_json::from_slice(&std::fs::read(directory.join("p11-envelopes.json"))?)?;
    if [&transcript, &malformed]
        .iter()
        .any(|bytes| bytes.is_empty() || bytes.len() > MAX_CAPTURE_BYTES)
    {
        return Err("P11 capture has invalid length".into());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let stream = tokio::io::join(Cursor::new(transcript.as_slice()), tokio::io::sink());
            let admission = Admission {
                appearance_items: None,
                schema_revision: 1,
                character_id: crate::foundation::CharacterId::decode(&character)?,
                // The bootstrap goes to the sink. No live admission grant is recorded.
                admission_material: b"offline-replay",
                client_build_id: "oteryn-dev-client/world-qualification",
                supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
                deadline: Duration::from_secs(20),
            };
            let mut session = Session::admit(stream, admission.clone()).await?;
            let replay = observe(&mut session).await?;
            if command_envelopes(&transcript, replay.session)? != envelopes
                || provenance(&transcript, &replay, &character, &envelopes, &malformed)? != expected
            {
                return Err(
                    "P11 replay did not match live session, commands, sequences or full page"
                        .into(),
                );
            }
            let negative_stream = || tokio::io::join(Cursor::new(malformed.as_slice()), tokio::io::sink());
            let mut refusal = Session::admit(negative_stream(), admission.clone()).await?;
            use oteryn_dev_client::client_runtime::account_achievements::Response;
            use oteryn_dev_client::SessionError;
            let empty = Response::Page(wire::AccountAchievementsResult::default());
            if refusal.join_snapshot().game_session_id == session.join_snapshot().game_session_id
                || refusal.account_achievements(0).await? != empty
                || refusal.account_achievements(0).await? != Response::Refused
                || refusal.account_achievements(0).await? != empty
            {
                return Err("P11 original server refusal did not preserve typed page access".into());
            }

            // Ask for page 1 while the original server reply still names page 0.
            let mut wrong_page = Session::admit(negative_stream(), admission).await?;
            if !matches!(
                wrong_page.account_achievements(1).await,
                Err(SessionError::Protocol(
                    oteryn_protocol_oteryn::FoundationProtocolError::MalformedFrame
                ))
            ) || !matches!(
                wrong_page.account_achievements(0).await,
                Err(SessionError::SessionUnusable)
            ) {
                return Err("P11 accepted a mismatched page or left its session usable".into());
            }
            Ok(())
        })
}

async fn fresh_token(clients: &SeamClients<'_>, index: usize) -> TestResult<String> {
    wait_character_released(clients, index).await?;
    let token = if requires_platform_grant(&clients.accounts[index]) {
        platform_row_grant(clients, index).await?
    } else {
        let generation = platform_generation(clients.descriptor, &clients.accounts[index]).await?;
        let (nonce, attempt) = fresh_grant_material(0x2f)?;
        sign_grant(
            &Grant {
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
            },
            now_seconds()?,
        )
    };
    Ok(token)
}

async fn read_wire_frame(stream: &mut (impl AsyncRead + Unpin)) -> TestResult<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let length = stream.read_u32().await?;
    let length = oteryn_protocol_oteryn::FrameLength::new(length)?;
    let mut bytes = vec![0; length.get() as usize];
    stream.read_exact(&mut bytes).await?;
    Ok(bytes)
}

fn reject_earned_wire(payload: &[u8]) -> TestResult {
    let delta = oteryn_protocol_oteryn::decode_state_delta(payload)?;
    if delta.domain_id
        == oteryn_protocol_oteryn::achievement_notices::STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES
    {
        return Err("P11 emitted an achievement notice before the grant".into());
    }
    Ok(())
}

#[test]
fn p11_rejects_a_premature_notice_in_the_raw_wire_probe() -> TestResult {
    use oteryn_protocol_oteryn::{achievement_notices, encode_state_delta};
    let ordinary = encode_state_delta(1, 1, 1, 0, 1, 1, &[8, 1])?;
    let premature = encode_state_delta(
        1, 1, achievement_notices::STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
        0, 1, achievement_notices::DELTA_TYPE_ACHIEVEMENT_EARNED_V1, &[8, 1],
    )?;
    reject_earned_wire(decode_wire_envelope(&ordinary)?.payload())?;
    assert!(reject_earned_wire(decode_wire_envelope(&premature)?.payload()).is_err());
    Ok(())
}

pub(super) async fn malformed_live(
    clients: &SeamClients<'_>,
    index: usize,
) -> TestResult<Vec<u8>> {
    use oteryn_protocol_oteryn::{
        MessageType, decode_command_result, decode_server_accepted, logout,
    };
    use tokio::io::AsyncWriteExt;
    let token = fresh_token(clients, index).await?;
    let tcp = tokio::net::TcpStream::connect(clients.address).await?;
    let stream = connector(clients.certificate, &EXACT)?
        .connect(
            tokio_rustls::rustls::pki_types::ServerName::try_from("localhost")?,
            tcp,
        )
        .await?;
    let bytes = Mutex::new(Vec::new());
    let mut stream = Recording { stream, bytes: &bytes };
    stream
        .write_all(&framed(&bootstrap_supporting(
            1,
            1,
            &clients.characters[index],
            &token,
            oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
        )))
        .await?;
    stream.flush().await?;
    let first = read_wire_frame(&mut stream).await?;
    let envelope = decode_wire_envelope(&first)?;
    if envelope.message_type() != MessageType::ServerAccepted {
        return Err("P11 malformed test admission refused".into());
    }
    let admitted = decode_server_accepted(envelope.payload())?;
    while decode_wire_envelope(&read_wire_frame(&mut stream).await?)?.message_type()
        != MessageType::SnapshotCommit
    {}
    let mut id = admitted.next_command_id;
    let mut pages = Vec::with_capacity(2);
    let mut rejected_at: Option<std::time::Instant> = None;
    for (kind, payload, status) in [
        (
            wire::COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY,
            Vec::new(),
            CommandStatus::Accepted,
        ),
        (
            wire::COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY,
            vec![0x08],
            CommandStatus::Rejected,
        ),
        (
            wire::COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY,
            Vec::new(),
            CommandStatus::Accepted,
        ),
        (
            logout::COMMAND_TYPE_LOGOUT_INTENT,
            logout::encode_logout_intent(),
            CommandStatus::Accepted,
        ),
    ] {
        stream
            .write_all(&framed(&client_command_at(
                admitted.connection_generation,
                id,
                u64::from(kind),
                &payload,
            )))
            .await?;
        stream.flush().await?;
        loop {
            let frame = read_wire_frame(&mut stream).await?;
            let envelope = decode_wire_envelope(&frame)?;
            match envelope.message_type() {
                MessageType::StateDelta => {
                    reject_earned_wire(envelope.payload())?;
                    continue;
                }
                MessageType::CommandResult => {}
                actual => {
                    return Err(
                        format!("P11 malformed test unexpected server frame: {actual:?}").into(),
                    );
                }
            }
            let result = decode_command_result(envelope.payload())?;
            if kind == logout::COMMAND_TYPE_LOGOUT_INTENT {
                let elapsed = rejected_at
                    .ok_or("P11 malformed rejection was never observed")?
                    .elapsed();
                evidence(&format!(
                    "P11_CHECK first_logout command={id} sequence={} status={:?} payload={:02x?} elapsed_since_rejection_ms={}",
                    envelope.server_sequence(),
                    result.status,
                    result.payload,
                    elapsed.as_millis()
                ));
            }
            if result.command_id != id || result.status != status || result.error_code != 0 {
                return Err(format!(
                    "P11 malformed test command={id} expected={status:?} actual={result:?}"
                )
                .into());
            }
            match (kind, status) {
                (_, CommandStatus::Rejected) if result.payload.is_empty() => {
                    rejected_at = Some(std::time::Instant::now());
                    evidence(&format!(
                        "P11_CHECK malformed_wire=Rejected command={id} sequence={} payload=08 error_code=0",
                        envelope.server_sequence()
                    ));
                }
                (logout::COMMAND_TYPE_LOGOUT_INTENT, _)
                    if logout::decode_logout_result(result.payload).ok()
                        == Some(logout::LogoutResult::Accepted) => {}
                (wire::COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY, CommandStatus::Accepted) => {
                    pages.push(
                        wire::decode_account_achievements_result(result.payload).map_err(
                            |error| format!("P11 malformed test page decode: {error:?}"),
                        )?,
                    );
                }
                _ => return Err("P11 malformed test unexpected result payload".into()),
            }
            break;
        }
        id = id
            .checked_add(1)
            .ok_or("P11 malformed test command id overflow")?;
    }
    if pages.as_slice()
        != [
            wire::AccountAchievementsResult::default(),
            wire::AccountAchievementsResult::default(),
        ]
    {
        return Err(format!("P11 malformed test changed the pre-grant page: {pages:?}").into());
    }
    drop(stream);
    bytes.into_inner().map_err(|_| "P11 malformed capture lock poisoned".into())
}

async fn journey(clients: &SeamClients<'_>, core: &SeamClients<'_>) -> TestResult<String> {
    full_row_p11::journey(clients, core, &std::cell::Cell::new(None)).await
}

pub(super) async fn capture_relog(
    clients: &SeamClients<'_>,
    proof: full_row_p11::GrantProof,
    malformed: Vec<u8>,
) -> TestResult<String> {
    let index = 1;
    let token = fresh_token(clients, index).await?;
    let admission = Admission {
        appearance_items: None,
        schema_revision: 1,
        character_id: crate::foundation::CharacterId::decode(&clients.characters[index])?,
        admission_material: token.as_bytes(),
        client_build_id: "oteryn-dev-client/world-qualification",
        supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
        deadline: Duration::from_secs(20),
    };
    let tcp = tokio::net::TcpStream::connect(clients.address).await?;
    let stream = connector(clients.certificate, &EXACT)?
        .connect(
            tokio_rustls::rustls::pki_types::ServerName::try_from("localhost")?,
            tcp,
        )
        .await?;
    let bytes = Mutex::new(Vec::new());
    let mut client = Session::admit(
        Recording {
            stream,
            bytes: &bytes,
        },
        admission,
    )
    .await?;
    if full_row_p11::inventory_count(client.inventory(), proof.reward) != Some(1) {
        return Err("P11 relog inventory did not contain exactly one Demon Armor".into());
    }
    let live = observe(&mut client).await?;
    if live.session == proof.session || live.page != proof.page {
        return Err("P11 new session did not load the exact earned achievement".into());
    }
    drop(client);
    full_row_p11::probe_relog_claim(clients, &proof).await?;
    let directory = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .ok_or("P11 requires the wrapper's fresh capture directory")?;
    let directory = Path::new(&directory);
    let transcript = bytes
        .into_inner()
        .map_err(|_| "P11 capture lock poisoned")?;
    let envelopes = command_envelopes(&transcript, live.session)?;
    let metadata = provenance(&transcript, &live, &clients.characters[index], &envelopes, &malformed)?;
    let envelopes_json = serde_json::to_vec(&envelopes)?;
    use std::io::Write;
    for (name, data) in [
        ("p11-frames.bin", transcript.as_slice()),
        ("p11-malformed-frames.bin", malformed.as_slice()),
        ("p11-envelopes.json", envelopes_json.as_slice()),
        ("p11-provenance.json", metadata.as_bytes()),
    ] {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(name))?
            .write_all(data)?;
    }
    evidence(&format!("P11_CAPTURE provenance={metadata}"));
    Ok(format!(
        "session={};page={};facts={};points={};malformed_wire=Rejected",
        uuid_text(&live.session),
        live.page.page,
        live.page.fact_count,
        live.page.total_points,
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, core: &SeamClients<'_>) {
    let start = std::time::Instant::now();
    let result = verdict::indexed_timeout(
        full_row_p11::JOURNEY_DEADLINE + Duration::from_secs(30),
        verdict::observe(journey(clients, core)),
    )
    .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("CAPTURED", detail),
        Ok(Err(error)) => ("FAIL", error.to_string()),
        Err(_) => ("FAIL", "timeout".to_owned()),
    };
    let kind = if status == "CAPTURED" {
        "e2e_capture"
    } else {
        "e2e_all"
    };
    evidence(&format!(
        "{kind} scenario=P11.client status={status} seconds={:.3} evidence={detail}",
        start.elapsed().as_secs_f64()
    ));
}
