//! Replay the original server frames through Session's typed achievement reader.
use super::*;
use oteryn_dev_client::native_session::{Admission, Session, SessionStream};
use oteryn_protocol_oteryn::account_achievements as wire;
use std::io::{self, Cursor};
use std::os::unix::fs::DirBuilderExt;
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
    Ok(Observation {
        session,
        commands: [first, next, pagination],
        sequences: [sequence, reread, paginated],
        page,
    })
}

// The aggregate qualification runner also consumes the original page payload.
fn export_page(transcript: &[u8], live: &Observation, directory: &Path) -> TestResult {
    use oteryn_protocol_oteryn::{CommandStatus, MessageType, decode_command_result};
    let Reply::Frames(records) = frames(transcript) else {
        return Err("P11 capture has no server frames".into());
    };
    for record in records {
        let envelope = decode_wire_envelope(&record)?;
        if envelope.message_type() != MessageType::CommandResult {
            continue;
        }
        let result = decode_command_result(envelope.payload())?;
        if result.command_id != live.commands[0] {
            continue;
        }
        if result.status != CommandStatus::Accepted {
            return Err("P11 first captured page result was not accepted".into());
        }
        std::fs::write(directory.join("p11-page.bin"), result.payload)?;
        let rows: Vec<_> = live
            .page
            .rows
            .iter()
            .map(|row| format!("{}:{}:{}", row.key, row.grade, row.points))
            .collect();
        std::fs::write(
            directory.join("p11-expected.txt"),
            format!(
                "page={}\ntotal_points={}\nfact_count={}\nhas_more={}\nrows={}\nmalformed_query=local-codec-refusal\nmalformed_wire_submission=false\n",
                live.page.page,
                live.page.total_points,
                live.page.fact_count,
                live.page.has_more,
                rows.join("|")
            ),
        )?;
        return Ok(());
    }
    Err("P11 capture is missing the initial page result".into())
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

async fn malformed_live(clients: &SeamClients<'_>, index: usize) -> TestResult {
    use oteryn_protocol_oteryn::{
        MessageType, decode_command_result, decode_server_accepted, logout,
    };
    use tokio::io::AsyncWriteExt;
    let token = fresh_token(clients, index).await?;
    let tcp = tokio::net::TcpStream::connect(clients.address).await?;
    let mut stream = connector(clients.certificate, &EXACT)?
        .connect(
            tokio_rustls::rustls::pki_types::ServerName::try_from("localhost")?,
            tcp,
        )
        .await?;
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
                MessageType::StateDelta => continue,
                MessageType::CommandResult => {}
                actual => {
                    return Err(
                        format!("P11 malformed test unexpected server frame: {actual:?}").into(),
                    );
                }
            }
            let result = decode_command_result(envelope.payload())?;
            if result.command_id != id || result.status != status || result.error_code != 0 {
                return Err(format!(
                    "P11 malformed test command={id} expected={status:?} actual={result:?}"
                )
                .into());
            }
            match (kind, status) {
                (_, CommandStatus::Rejected) if result.payload.is_empty() => {
                    evidence(&format!(
                        "P11_CHECK malformed_wire=Rejected command={id} sequence={}",
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
    Ok(())
}

async fn journey(clients: &SeamClients<'_>, core: &SeamClients<'_>) -> TestResult<String> {
    let index = 1;
    malformed_live(clients, index).await?;
    let proof = full_row_p11::earn_and_logout(clients, core).await?;
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
    let live = observe(&mut client).await?;
    if live.session == proof.session || live.page != proof.page {
        return Err("P11 new session did not load the exact earned achievement".into());
    }
    drop(client);
    let session = uuid_text(&live.session);
    let root = std::env::temp_dir().join(format!("oteryn-p11-{session}"));
    std::fs::DirBuilder::new().mode(0o700).create(&root)?;
    let result = async {
        let capture = root.join("server.frames");
        let captured = bytes
            .into_inner()
            .map_err(|_| "P11 capture lock poisoned")?;
        std::fs::write(&capture, &captured)?;
        let input = std::fs::read(capture)?;
        if input != captured {
            return Err("P11 capture changed before replay".into());
        }
        let stream = tokio::io::join(Cursor::new(input), tokio::io::sink());
        let mut replay = Session::admit(stream, admission).await?;
        if observe(&mut replay).await? != live {
            return Err("P11 replay changed the session, continuity or achievement page".into());
        }
        if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            export_page(&captured, &live, Path::new(&directory))?;
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;
    let cleanup = std::fs::remove_dir_all(root);
    result?;
    cleanup?;
    evidence(&format!(
        "P11_CHECK replay=wire PASS capture=current-run session={session} commands={:?} sequences={:?}",
        live.commands, live.sequences,
    ));
    Ok(format!(
        "session={session};page={};facts={};points={};malformed_query=local-codec-refusal;malformed_wire_submission=false",
        live.page.page, live.page.fact_count, live.page.total_points,
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, core: &SeamClients<'_>) {
    let start = std::time::Instant::now();
    let result = verdict::indexed_timeout(
        Duration::from_secs(120),
        verdict::observe(journey(clients, core)),
    )
    .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", detail),
        Ok(Err(error)) => ("FAIL", error.to_string()),
        Err(_) => ("FAIL", "timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=P11.client status={status} seconds={:.3} evidence={detail}",
        start.elapsed().as_secs_f64()
    ));
}
