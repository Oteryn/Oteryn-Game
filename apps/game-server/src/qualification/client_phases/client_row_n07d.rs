//! Captures two live admissions and the first-attempt logout between them.
use super::*;
use oteryn_dev_client::native_session::{Admission, Session};
use oteryn_protocol_oteryn::world_map::{
    STATE_DOMAIN_WORLD_MAP_VIEW, VIEW_HEIGHT, VIEW_LEFT, VIEW_TOP, VIEW_WIDTH,
    WorldMapViewSnapshot, in_window,
};
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

fn viewport_ground(
    world: &BundleWorld,
    map: &WorldMapViewSnapshot,
    expected: ActorPosition,
) -> TestResult<Vec<(i16, i32, i32)>> {
    if map.header.origin != expected || map.header.first_visible_floor != expected.floor {
        return Err(format!(
            "N07d wrong viewport: expected={expected:?} actual={:?}",
            map.header
        )
        .into());
    }
    let mut ground = Vec::new();
    for x in expected.x - VIEW_LEFT..expected.x - VIEW_LEFT + VIEW_WIDTH {
        for y in expected.y - VIEW_TOP..expected.y - VIEW_TOP + VIEW_HEIGHT {
            let at = tile(ActorPosition {
                x,
                y,
                floor: expected.floor,
            })?;
            if world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| tile.stored_ground_speed() > 0)
            {
                ground.push((expected.floor, x, y));
            }
        }
    }
    let observed = map
        .tiles
        .iter()
        .filter(|tile| {
            tile.ground_speed > 0
                && tile.position.floor >= map.header.first_visible_floor
                && in_window(expected, tile.position)
        })
        .map(|tile| (tile.position.floor, tile.position.x, tile.position.y))
        .collect::<std::collections::BTreeSet<_>>();
    if !ground.iter().copied().eq(observed.iter().copied()) || ground.is_empty() {
        return Err(format!("N07d viewport ground differs from pinned World: origin={expected:?} expected={} actual={}", ground.len(), observed.len()).into());
    }
    Ok(ground)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let capture_dir = required("E2E_ALL_CAPTURE_DIR")?;
    let root = Path::new(&capture_dir);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let origin = ActorPosition {
        x: i32::from(TOWN.x),
        y: i32::from(TOWN.y),
        floor: -i16::from(TOWN.floor),
    };
    let destination = ActorPosition {
        x: origin.x + 1,
        ..origin
    };
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
    let initial_ground = viewport_ground(world, &map, origin)?;
    if client.world_spatial().actor_position != origin {
        return Err("N07d initial spatial position differs from town".into());
    }
    let walkable = initial_ground.len();
    let step = client
        .step(oteryn_protocol_oteryn::world_spatial::StepDirection::East)
        .await?;
    if step.status != oteryn_protocol_oteryn::CommandStatus::Accepted
        || client.world_spatial().actor_position != destination
    {
        return Err(format!("N07d first-attempt east step failed: {step:?}").into());
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        while !client
            .world_map_snapshot()
            .is_some_and(|map| map.header.origin == destination)
        {
            client.wait_for_events(Duration::from_secs(5)).await?;
        }
        Ok::<_, oteryn_dev_client::SessionError>(())
    })
    .await??;
    let step_capture = envelope(
        &first_id,
        step.command_id,
        step.result_server_sequence,
        captured_bytes(&incoming)?,
    )?;
    let moved = client
        .world_map_snapshot()
        .ok_or("N07d moved viewport missing")?;
    let moved_ground = viewport_ground(world, &moved, destination)?;
    if !initial_ground
        .iter()
        .any(|&(floor, x, y)| !in_window(destination, ActorPosition { floor, x, y }))
    {
        return Err("N07d step left no previously observed ground outside relog viewport".into());
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
    let relog_map = relogged
        .world_map_snapshot()
        .ok_or("N07d relogged viewport missing")?;
    if relogged.world_spatial().actor_position != destination
        || viewport_ground(world, &relog_map, destination)? != moved_ground
    {
        return Err("N07d relog did not retain the exact destination viewport".into());
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
    std::fs::write(root.join("n07d-step.json"), step_capture)?;
    std::fs::write(
        root.join("n07d-ground.json"),
        serde_json::to_vec(&(initial_ground, moved_ground))?,
    )?;
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
