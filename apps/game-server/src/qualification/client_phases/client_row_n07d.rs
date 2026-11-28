//! Captures two live admissions and the first-attempt logout between them.
use super::*;
use oteryn_dev_client::native_session::{Admission, Session};
use oteryn_protocol_oteryn::world_map::{
    STATE_DOMAIN_WORLD_MAP_VIEW, VIEW_HEIGHT, VIEW_LEFT, VIEW_TOP, VIEW_WIDTH,
    WorldMapViewSnapshot, floors_in_view, in_window,
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
    initial_admission: bool,
) -> TestResult<(
    Session<Recorded<impl AsyncRead + AsyncWrite + Unpin>>,
    Recording,
)> {
    const INDEX: usize = 1;
    if initial_admission {
        wait_character_released(clients, INDEX).await?;
    }
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
            // Negotiate the map, movement and logout profile on both live admissions.
            // Unrelated panels can send automatic queries before the first movement.
            supported_capabilities: &[4, 6, 11, 13, 15, 18, 21, 30],
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
    let floors = floors_in_view(expected.floor).ok_or("N07d invalid viewport floor")?;
    for floor in floors {
        let shift = i32::from(expected.floor - floor);
        let left = expected.x - VIEW_LEFT + shift;
        let top = expected.y - VIEW_TOP + shift;
        for x in left..left + VIEW_WIDTH {
            for y in top..top + VIEW_HEIGHT {
                let at = tile(ActorPosition { x, y, floor })?;
                if world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| tile.stored_ground_speed() > 0)
                {
                    ground.push((floor, x, y));
                }
            }
        }
    }
    let observed = map
        .tiles
        .iter()
        .filter(|tile| tile.ground_speed > 0 && in_window(expected, tile.position))
        .map(|tile| ((tile.position.floor, tile.position.x, tile.position.y), tile))
        .collect::<std::collections::BTreeMap<_, _>>();
    if !ground.iter().copied().eq(observed.keys().copied()) || ground.is_empty() {
        return Err(format!("N07d viewport ground differs from pinned World: origin={expected:?} expected={} actual={}", ground.len(), observed.len()).into());
    }
    verify_ground_speeds(observed.iter().map(|(&key, tile)| (key, tile.ground_speed)))?;
    // Bind the client's pinned appearance colours to the pinned World stack at this position.
    // The fixed walk never changes a tile entry, so a different served stack is an error.
    use oteryn_gameplay_world::map::view::{compose_into, cut_indices};
    let mut entries = Vec::new();
    for (key, snapshot_tile) in observed {
        entries.clear();
        compose_into(
            world.base(),
            None,
            world.facts(),
            tile(snapshot_tile.position)?,
            &mut entries,
        ).map_err(|error| format!("N07d pinned terrain stack at {key:?}: {error:?}"))?;
        let (kept, more) = cut_indices(entries.len());
        if more != snapshot_tile.more || !kept
            .map(|index| entries[index].facts.appearance_id)
            .eq(snapshot_tile.items.iter().map(|item| item.appearance_id))
        {
            return Err(format!("N07d appearance stack differs from pinned World at {key:?}").into());
        }
    }
    ground.retain(|&(floor, _, _)| floor >= map.header.first_visible_floor);
    Ok(ground)
}

fn verify_ground_speeds(
    observed: impl IntoIterator<Item = ((i16, i32, i32), u16)>,
) -> TestResult<()> {
    // Domain 17 uses the Engineering pacing speed for every pinned ground tile.
    // The bundle's stored speed determines presence, while the wire contract fixes the value.
    const ENGINEERING_GROUND_SPEED: u16 = 150;
    for (position, speed) in observed {
        if speed != ENGINEERING_GROUND_SPEED {
            return Err(format!("N07d viewport ground speed at {position:?}: expected={ENGINEERING_GROUND_SPEED} actual={speed}").into());
        }
    }
    Ok(())
}

#[test]
fn n07d_rejects_a_wrong_positive_ground_speed() -> TestResult<()> {
    let at = (7, 32361, 32232);
    verify_ground_speeds([(at, 150)])?;
    let result = verify_ground_speeds([(at, 151)]);
    assert_eq!(
        result.err().map(|error| error.to_string()).as_deref(),
        Some("N07d viewport ground speed at (7, 32361, 32232): expected=150 actual=151")
    );
    Ok(())
}

fn require_logout_commit(session: &str, terminal: bool, loss_seen: bool) -> TestResult {
    if !terminal || loss_seen {
        return Err(format!(
            "N07d accepted logout did not commit release before disconnect: session={session} terminal={terminal} loss_seen={loss_seen}"
        ).into());
    }
    Ok(())
}

#[test]
fn n07d_logout_proof_rejects_early_acknowledgement_and_loss_recovery() -> TestResult {
    require_logout_commit("fixture", true, false)?;
    assert!(require_logout_commit("fixture", false, false).is_err());
    assert!(require_logout_commit("fixture", true, true).is_err());
    Ok(())
}

async fn assert_logout_committed(clients: &SeamClients<'_>, session: &str) -> TestResult {
    let (terminal, loss_seen) = tokio::time::timeout(Duration::from_secs(5), async {
        let mut connection = sqlx::PgConnection::connect(clients.url).await?;
        sqlx::query_as::<_, (bool, bool)>(
            "SELECT (s.session_state = 3 AND s.current_transport_ref IS NULL \
                AND NOT EXISTS (SELECT 1 FROM game_durability_admission_character_guards c \
                    WHERE c.character_id = s.character_id AND c.holder_game_session_id = s.game_session_id) \
                AND NOT EXISTS (SELECT 1 FROM game_durability_admission_account_guards a \
                    WHERE a.account_id = s.account_id AND a.presence_character_id = s.character_id)), \
                (s.control_loss_epoch IS NOT NULL OR s.original_grace_deadline IS NOT NULL) \
             FROM game_durability_reconnect_sessions s \
             WHERE s.game_session_id = $1::text::uuid AND s.world_id = encode($2,'hex')::uuid \
               AND s.character_id = encode($3,'hex')::uuid",
        )
        .bind(session)
        .bind(clients.world.as_slice())
        .bind(clients.characters[1].as_slice())
        .fetch_one(&mut connection)
        .await
    })
    .await??;
    require_logout_commit(session, terminal, loss_seen)?;
    evidence(&format!("N07d logout_commit session={session} terminal=true loss_seen=false socket=held"));
    Ok(())
}

fn assert_cache_identity(
    actual: ([u8; 32], [u8; 32], u64),
    expected: ([u8; 32], [u8; 32], u64),
) -> TestResult {
    if actual != expected {
        return Err(format!("N07d cache identity differs from pinned World and activation: actual={actual:?} expected={expected:?}").into());
    }
    Ok(())
}

#[test]
fn n07d_rejects_matching_logins_with_wrong_cache_identity() -> TestResult {
    let pinned = ([1; 32], [2; 32], 0);
    assert_cache_identity(pinned, pinned)?;
    for wrong in [([3; 32], pinned.1, 0), (pinned.0, [3; 32], 0), (pinned.0, pinned.1, 1)] {
        assert!(assert_cache_identity(wrong, pinned).is_err());
        assert!(assert_cache_identity(wrong, pinned).is_err());
    }
    Ok(())
}

async fn pinned_cache_identity(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult<([u8; 32], [u8; 32], u64)> {
    let (generation, epoch) = tokio::time::timeout(Duration::from_secs(5), async {
        let mut connection = sqlx::PgConnection::connect(clients.url).await?;
        sqlx::query_as::<_, (Vec<u8>, String)>(
            "SELECT a.client_artifact_digest, r.reset_epoch::text \
             FROM game_content_activations a JOIN game_world_reset_active r USING (world_id) \
             WHERE a.world_id=encode($1,'hex')::uuid AND a.channel_id=encode($2,'hex')::uuid \
               AND a.activation_sequence=1",
        )
        .bind(clients.world.as_slice())
        .bind(clients.channel.as_slice())
        .fetch_one(&mut connection)
        .await
    }).await??;
    let epoch: u64 = epoch.parse()?;
    if epoch != 0 {
        return Err(format!("N07d expected the fixture's initialized epoch zero, observed {epoch}").into());
    }
    let generation = generation.try_into().map_err(|_| "N07d activation digest has invalid width")?;
    Ok((generation, world.base().digest(), epoch))
}

fn hex_digest(digest: &[u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn map_cache_identity(map: &WorldMapViewSnapshot) -> ([u8; 32], [u8; 32], u64) {
    (map.header.content_generation, map.header.bundle_digest, map.header.reset_epoch)
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
    let cache_identity = pinned_cache_identity(clients, world).await?;
    let origin = ActorPosition {
        x: i32::from(TOWN.x),
        y: i32::from(TOWN.y),
        floor: -i16::from(TOWN.floor),
    };
    let destination = ActorPosition {
        x: origin.x + 1,
        ..origin
    };
    let (mut client, incoming) = admit(clients, 0x3d, true).await?;
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
    assert_cache_identity(map_cache_identity(&map), cache_identity)?;
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
        || step.disposition != oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
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
    assert_cache_identity(map_cache_identity(&moved), cache_identity)?;
    let moved_ground = viewport_ground(world, &moved, destination)?;
    if !initial_ground
        .iter()
        .any(|&(floor, x, y)| !in_window(destination, ActorPosition { floor, x, y }))
    {
        return Err("N07d step left no previously observed ground outside relog viewport".into());
    }
    let expected = format!(
        "revision={}\norigin={},{},{}\nfirst_visible_floor={}\nreset_epoch={}\ncontent_generation={}\nbundle_digest={}\ntiles={}\nwalkable={walkable}\n",
        row.revision,
        header.origin.x,
        header.origin.y,
        header.origin.floor,
        header.first_visible_floor,
        cache_identity.2,
        hex_digest(&cache_identity.0),
        hex_digest(&cache_identity.1),
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
    assert_logout_committed(clients, &first_id).await?;
    drop(client);
    let (relogged, incoming) = admit(clients, 0x3e, false).await?;
    let second_id = uuid_text(relogged.join_snapshot().game_session_id.as_bytes());
    if first_id == second_id {
        return Err("N07d relog retained the old session".into());
    }
    let relog_map = relogged
        .world_map_snapshot()
        .ok_or("N07d relogged viewport missing")?;
    assert_cache_identity(map_cache_identity(&relog_map), cache_identity)?;
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
