//! Captures two live admissions and the first-attempt logout between them.
use super::*;
use oteryn_dev_client::native_session::{Admission, Session};
use oteryn_protocol_oteryn::world_map::{STATE_DOMAIN_WORLD_MAP_VIEW, in_window};
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

type Recording = Arc<Mutex<Vec<u8>>>;

struct Recorded<T> {
    stream: T,
    incoming: Recording,
}

impl<T: AsyncRead + Unpin> AsyncRead for Recorded<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let result = Pin::new(&mut self.stream).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = result {
            self.incoming
                .lock()
                .expect("recording lock")
                .extend_from_slice(&buf.filled()[before..]);
        }
        result
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for Recorded<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

async fn admit(
    clients: &SeamClients<'_>,
    tag: u8,
) -> TestResult<(
    Session<Recorded<impl AsyncRead + AsyncWrite + Unpin>>,
    Recording,
)> {
    const INDEX: usize = 1;
    wait_character_released(clients, INDEX).await?;
    let token = if requires_platform_grant(&clients.accounts[INDEX]) {
        platform_row_grant(clients, INDEX).await?
    } else {
        let (nonce, attempt) = fresh_grant_material(tag)?;
        sign_grant(
            &Grant {
                signing: clients.signing,
                key_id: clients.key_id,
                account_id: &clients.accounts[INDEX],
                character_id: clients.characters[INDEX],
                world_id: clients.world,
                channel_id: clients.channel,
                security_generation: platform_generation(
                    clients.descriptor,
                    &clients.accounts[INDEX],
                )
                .await?,
                scope_generation: clients.scope_generation,
                nonce,
                attempt,
            },
            now_seconds()?,
        )
    };
    let stream = tokio::time::timeout(Duration::from_secs(20), async {
        let tcp = TcpStream::connect(clients.address).await?;
        Ok::<_, Box<dyn std::error::Error>>(
            connector(clients.certificate, &EXACT)?
                .connect(ServerName::try_from("localhost")?, tcp)
                .await?,
        )
    })
    .await??;
    let incoming = Recording::default();
    let session = Session::admit(
        Recorded {
            stream,
            incoming: incoming.clone(),
        },
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(&clients.characters[INDEX])?,
            admission_material: token.as_bytes(),
            client_build_id: "oteryn-dev-client/world-qualification",
            supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(20),
        },
    )
    .await?;
    verdict::Harness::global().progress("admission");
    Ok((session, incoming))
}

fn captured_bytes(recording: &Recording) -> TestResult<Vec<u8>> {
    Ok(std::mem::take(
        &mut *recording.lock().map_err(|_| "recording lock poisoned")?,
    ))
}

// The five-key #3521 envelope matches the N03 writer. In an admission envelope,
// command_id is the next command and server_sequence is the installed baseline.
fn envelope(
    session: &str,
    command_id: u64,
    server_sequence: u64,
    payload: Vec<u8>,
) -> TestResult<Vec<u8>> {
    if payload.is_empty() {
        return Err("N07d wire recording is empty".into());
    }
    Ok(serde_json::to_vec(&serde_json::json!({
        "command_id": command_id,
        "game_session_id": session,
        "payload": payload,
        "server_sequence": server_sequence,
        "status": oteryn_protocol_oteryn::CommandStatus::Accepted as u32,
    }))?)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let capture_dir = required("E2E_ALL_CAPTURE_DIR")?;
    let root = Path::new(&capture_dir);
    let (mut client, incoming) = admit(clients, 0x3d).await?;
    let first_id = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let first_capture = envelope(
        &first_id,
        client.next_command_id(),
        client.last_server_sequence(),
        captured_bytes(&incoming)?,
    )?;
    let row = client
        .gated_snapshot(STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("admission carried no world-map snapshot")?
        .clone();
    let map = client
        .world_map_snapshot()
        .ok_or("admitted world-map snapshot malformed")?;
    if row.payload.is_empty() {
        return Err("empty world-map snapshot payload".into());
    }
    let header = map.header;
    // Server-observed walkable ground inside the visible window (the cells the minimap remembers).
    let walkable = map
        .tiles
        .iter()
        .filter(|tile| {
            tile.ground_speed > 0
                && tile.position.floor >= header.first_visible_floor
                && in_window(header.origin, tile.position)
        })
        .count();
    if walkable == 0 {
        return Err("admitted map snapshot has no walkable ground".into());
    }
    let expected = format!(
        "revision={}\norigin={},{},{}\nfirst_visible_floor={}\nreset_epoch={}\ntiles={}\nwalkable={walkable}\n",
        row.revision,
        header.origin.x,
        header.origin.y,
        header.origin.floor,
        header.first_visible_floor,
        header.reset_epoch,
        map.tiles.len()
    );
    let logout = client.logout().await?;
    if logout.status != oteryn_protocol_oteryn::CommandStatus::Accepted
        || logout.result != Some(LogoutResult::Accepted)
    {
        return Err(format!("N07d first-attempt logout refused: {logout:?}").into());
    }
    let logout_capture = envelope(
        &first_id,
        logout.command_id,
        logout.result_server_sequence,
        captured_bytes(&incoming)?,
    )?;
    drop(client);
    let (relogged, incoming) = admit(clients, 0x3e).await?;
    let second_id = uuid_text(relogged.join_snapshot().game_session_id.as_bytes());
    if first_id == second_id {
        return Err("N07d relog retained the old session".into());
    }
    let second_capture = envelope(
        &second_id,
        relogged.next_command_id(),
        relogged.last_server_sequence(),
        captured_bytes(&incoming)?,
    )?;
    let expected = format!(
        "{expected}capture_dir={capture_dir}\nworld={}\ncharacter={}\nfirst_session={first_id}\nsecond_session={second_id}\n",
        uuid_text(&clients.world),
        uuid_text(&clients.characters[1])
    );
    std::fs::write(root.join("n07d-admission.json"), first_capture)?;
    std::fs::write(root.join("n07d-logout.json"), logout_capture)?;
    std::fs::write(root.join("n07d-relog.json"), second_capture)?;
    std::fs::write(root.join("n07d-snapshot.bin"), &row.payload)?;
    std::fs::write(root.join("n07d-expected.txt"), expected.as_bytes())?;
    Ok(format!(
        "first_session={first_id};second_session={second_id};walkable={walkable}"
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    match verdict::indexed_timeout(Duration::from_secs(100), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=N07d.client status=CAPTURED seconds={:.3} evidence={observed}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N07d.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=N07d.client status=FAIL evidence=timeout"),
    }
}
