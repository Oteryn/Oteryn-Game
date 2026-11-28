//! CLIENT-I09: rotate compiled, fixed chair 2358 through the client Rotate menu and observe the
//! domain-17 map delta that reports chair 2359. The replay owner is `oteryn-session`:
//! `crates/session/src/map_transport.rs::apply` decodes and validates the delta and queues the
//! client-visible `SessionEvent::WorldMap`; `Session::take_events` exposes it to the native client.
use super::*;
use oteryn_dev_client::{
    SessionEvent,
    world_interaction::{self, Action, Command, Selection, Target},
};
use oteryn_protocol_oteryn::world_map::{self, MapDefinition, MapOrigin};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};

const BEFORE: &str = "oteryn:item.tibia.i2358";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(
        Duration::from_secs(360),
        verdict::observe(oteryn_dev_client::with_qualification_transcript(journey(
            clients,
        ))),
    )
    .await;
    match result {
        Ok(Ok(())) => evidence(
            "SEAM_EVIDENCE e2e_capture scenario=I09.client status=CAPTURED evidence=domain-17-original-received-frames",
        ),
        Ok(Err(error)) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=I09.client status=FAIL evidence={}",
            error
                .to_string()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("-")
        )),
        Err(_) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=I09.client status=FAIL evidence=journey-timeout",
        ),
    }
}

fn capture(directory: &Path, name: &str, bytes: &[u8]) -> TestResult {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(name))?;
    std::io::Write::write_all(&mut file, bytes)?;
    Ok(())
}

// Only command-result frames carry a command ID and status on the wire.
fn capture_envelopes(root: &Path, name: &str, mut transcript: &[u8], session: &str) -> TestResult {
    use oteryn_protocol_oteryn::{
        FrameLength, MessageType, decode_command_result, decode_wire_envelope,
    };

    let mut envelopes = Vec::new();
    while !transcript.is_empty() {
        let length =
            FrameLength::from_prefix(transcript.get(..4).ok_or("short I09 frame prefix")?)?;
        let end = 4 + length.get() as usize;
        let wire = decode_wire_envelope(transcript.get(4..end).ok_or("short I09 frame")?)?;
        let sequence = wire.server_sequence();
        let (command_id, status) = if wire.message_type() == MessageType::CommandResult {
            let reply = decode_command_result(wire.payload())?;
            (reply.command_id, reply.status as u32)
        } else {
            (0, 0)
        };
        envelopes.push(serde_json::json!({
            "command_id": command_id,
            "game_session_id": session,
            "payload": wire.payload(),
            "server_sequence": sequence,
            "status": status,
        }));
        transcript = &transcript[end..];
    }
    capture(root, name, &serde_json::to_vec(&envelopes)?)
}

type MenuSelection = ((i32, i32, i16), Option<([u8; 32], [u8; 32], u64)>);

async fn rotate_menu_command(
    capture_root: &Path,
    map: &[u8],
    actor: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    target: oteryn_protocol_oteryn::world_spatial::ActorPosition,
) -> TestResult<Command> {
    let binary = required("OTERYN_I09_MENU_TEST")?;
    let map_path = capture_root.join("i09-rotate-menu-map.bin");
    capture(capture_root, "i09-rotate-menu-map.bin", map)?;
    let coordinates = format!("{} {} {} {} {}", actor.x, actor.y, actor.floor, target.x, target.y);
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(binary)
            .args(["qualification_i09_rotate_menu", "--ignored", "--nocapture"])
            .env("OTERYN_I09_MENU_MAP", map_path)
            .env("OTERYN_I09_CLICK", coordinates)
            .env("OTERYN_ASSET_DIR", Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .output()
    }).await??;
    let stdout = String::from_utf8(output.stdout)?;
    if !output.status.success() {
        return Err(format!("I09 Rotate context menu failed: {stdout} {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    let mut payloads = stdout.lines().filter_map(|line| line.strip_prefix("I09_MENU_USE="));
    let payload = payloads.next().ok_or("I09 Rotate context menu did not emit a command")?;
    if payloads.next().is_some() {
        return Err("I09 Rotate context menu emitted multiple commands".into());
    }
    let (placement, expected_revision, selection): (Vec<u8>, u64, Option<MenuSelection>) =
        serde_json::from_str(payload)?;
    Ok(Command {
        target: Target::Object(WorldObjectTarget { placement, expected_revision }),
        action: Action::Use,
        selection: selection.map(|((x, y, floor), binding)| Selection {
            position: oteryn_protocol_oteryn::world_spatial::ActorPosition { x, y, floor },
            binding,
        }),
    })
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let capture_root = required("E2E_ALL_CAPTURE_DIR")?;
    let capture_root = Path::new(&capture_root);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let at = TilePos {
        x: 32355,
        y: 32206,
        floor: -7,
    };
    if world.facts().house_id(at).is_some() {
        return Err("fixed chair 2358 belongs to a house".into());
    }
    let mut ordinal = None;
    oteryn_world_bundle::bundle::visit(&bytes, |manifest, sector| {
        for tile in sector.tiles {
            if (tile.x, tile.y, sector.floor) != (at.x, at.y, at.floor) {
                continue;
            }
            for (index, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if manifest.palette[item.palette as usize].key == BEFORE {
                    if ordinal.is_some() {
                        return Err(oteryn_world_bundle::Error::Limit(
                            "ambiguous I09 chair".into(),
                        ));
                    }
                    ordinal = Some(u8::try_from(index).map_err(|_| {
                        oteryn_world_bundle::Error::Limit("I09 furniture ordinal".into())
                    })?);
                }
            }
        }
        Ok(())
    })?;
    let ordinal = ordinal.ok_or("compiled I09 tile omitted fixed chair 2358")?;
    let definitions = provisioning::item_index()?;
    let expected = provisioning::item_reference(&definitions, 2359)?;
    let mut client = connect_world(clients, 0xc9).await?;
    let approach = TilePos { x: 32356, ..at };
    let path = route(
        world,
        tile(client.world_spatial().actor_position)?,
        |cell| cell == approach,
    )
    .ok_or("no route to fixed chair approach at 32356,32206,7")?;
    walk_once(&mut client, world, &path).await?;
    if tile(client.world_spatial().actor_position)? != approach {
        return Err("I09 arrived outside the fixed chair approach".into());
    }
    probe_chair_collision(&mut client, world, approach, at).await?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(
        &crate::map::view::placement_key(at, ordinal)
            .ok_or("chair placement")?
            .to_be_bytes(),
    );
    let intent = |revision| {
        encode_item_use_intent(&UseIntent {
            target: UseTarget::WorldObject(WorldObjectTarget {
                placement: placement.clone(),
                expected_revision: revision,
            }),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("I09 intent encode: {error:?}"))
    };
    let target = oteryn_protocol_oteryn::world_spatial::ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    };
    let before = client
        .session_mut()
        .gated_snapshot(world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("I09 predecessor map snapshot absent")?
        .clone();
    let before_map = world_map::decode_world_map_snapshot(&before.payload)
        .map_err(|error| format!("I09 predecessor decode: {error:?}"))?;
    let original = provisioning::item_reference(&definitions, 2358)?;
    let before_item = before_map.tiles.iter().find(|tile| tile.position == target)
        .and_then(|tile| tile.items.iter().find(|item| {
            item.origin == (MapOrigin::BaseOrdinal { ordinal, object_revision: 0 })
        }))
        .filter(|item| item.definition == MapDefinition::Item(original) && item.appearance_id == 2358)
        .ok_or("I09 predecessor omitted original chair at revision 0")?;
    require_unique_chair(&before_map.tiles, target, before_item)?;
    client.take_events();
    let command = rotate_menu_command(capture_root, &before.payload, client.world_spatial().actor_position, target).await?;
    let Target::Object(object) = &command.target else {
        return Err("I09 Rotate menu did not emit an object target".into());
    };
    let stale_payload = oteryn_protocol_oteryn::world_object::encode_use_intent(object)
        .map_err(|error| format!("I09 Rotate target encode: {error:?}"))?;
    let header = before_map.header;
    if stale_payload != intent(0)? || command.selection != Some(Selection {
        position: target,
        binding: Some((header.content_generation, header.bundle_digest, header.reset_epoch)),
    }) {
        return Err("I09 Rotate menu did not carry the exact public chair selection".into());
    }
    // Match the production world-context dispatch: service pending frames while
    // the menu owns input, then validate the live selection before submitting.
    client.service_liveness(Duration::from_millis(1)).await?;
    require_actor_tile(tile(client.world_spatial().actor_position)?, approach)?;
    assert_chair(&mut client, target, before_item)?;
    if client.take_events().iter().any(|event| matches!(
        event, SessionEvent::GatedSnapshotReplaced(update) if update.value.domain_id == before.domain_id
    )) {
        return Err("I09 map snapshot replaced while the Rotate menu owned input".into());
    }
    let before = client.session_mut().gated_snapshot(before.domain_id)
        .ok_or("I09 map snapshot absent before Rotate dispatch")?.clone();
    let session = client.session_mut();
    let use_command_id = session.next_command_id();
    let mut unselected = command.clone();
    unselected.selection = None;
    let refused = world_interaction::send_framed(session, &unselected, true).await?;
    if refused != "Target changed" || session.next_command_id() != use_command_id {
        return Err("I09 missing Rotate selection crossed the production send guard".into());
    }
    evidence("I09_MENU_ASSERTION missing-selection=Target-changed no-command-sent");
    let before_use_sequence = session.last_server_sequence();
    let outcome = world_interaction::send_framed(session, &command, true).await?;
    if outcome != "Use: completed" || use_command_id.checked_add(1) != Some(session.next_command_id()) {
        return Err(format!("I09 first native Rotate outcome: {outcome}").into());
    }
    let use_reply_sequence = session.last_server_sequence();
    evidence("I09_MENU_ASSERTION rotate=clicked-public-chair dispatch=production");
    let rotated = MapOrigin::BaseOrdinal {
        ordinal,
        object_revision: 1,
    };
    let started = tokio::time::Instant::now();
    let delta = loop {
        let mut rotation = None;
        for event in client.take_events() {
            match event {
                SessionEvent::GatedSnapshotReplaced(update)
                    if update.value.domain_id == before.domain_id =>
                {
                    return Err("I09 map snapshot replaced before rotation delta".into());
                }
                SessionEvent::WorldMap(delta)
                    if delta.value.cleared.contains(&target)
                        || delta.value.tiles.iter().any(|tile| tile.position == target) =>
                {
                    rotation = Some(delta);
                    break;
                }
                _ => {}
            }
        }
        if let Some(delta) = rotation {
            break delta;
        }
        // Match the full row's map observation budget. USE may publish on a later
        // visibility refresh, whose nominal cadence already exceeds 100 ms.
        if started.elapsed() >= Duration::from_secs(5) {
            return Err("rotation sent no map delta for the revised chair".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };
    if delta.server_sequence <= use_reply_sequence {
        return Err("I09 chair update did not follow the USE reply".into());
    }
    if delta.base_revision != before.revision {
        return Err("I09 predecessor revision does not match rotation delta".into());
    }
    evidence(&format!(
        "I09 rotation_delta_ms={}",
        started.elapsed().as_millis()
    ));
    let rotated_tile = delta
        .value
        .tiles
        .iter()
        .find(|tile| tile.position == target)
        .ok_or("rotation delta omitted target tile")?;
    let item = rotated_tile
        .items
        .iter()
        .find(|item| item.origin == rotated)
        .ok_or("rotation delta omitted revised furniture")?;
    // The client replay selects the revision-1 item from this delta.
    if delta
        .value
        .tiles
        .iter()
        .flat_map(|tile| &tile.items)
        .filter(|item| {
            matches!(
                item.origin,
                MapOrigin::BaseOrdinal {
                    object_revision: 1,
                    ..
                }
            )
        })
        .count()
        != 1
    {
        return Err("I09 rotation delta has ambiguous revision-1 items".into());
    }
    let MapDefinition::Item(definition) = item.definition else {
        return Err("rotated furniture was not an item".into());
    };
    if *item != (world_map::MapItem {
        definition: MapDefinition::Item(expected),
        appearance_id: 2359,
        count: 1,
        origin: MapOrigin::BaseOrdinal { ordinal, object_revision: 1 },
        ..*before_item
    }) {
        return Err(format!(
            "rotation expected definition {expected} and appearance 2359, observed {item:?}"
        )
        .into());
    }
    require_unique_chair(&delta.value.tiles, target, item)?;
    let stale = client.session_mut().world_context_use(&stale_payload).await?;
    if stale.status != CommandStatus::Accepted
        || decode_use_result(&stale.payload)
            .map_err(|error| format!("I09 stale decode: {error:?}"))?
            != UseDisposition::StaleState
    {
        return Err("stale rotation was not refused".into());
    }
    assert_chair(&mut client, target, item)?;
    probe_chair_collision(&mut client, world, approach, at).await?;
    assert_chair(&mut client, target, item)?;
    let session = client.join_snapshot().game_session_id;
    let position = client.world_spatial().actor_position;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let initial_id = uuid_text(session.as_bytes());
    require_release_state(&mut database, &initial_id, clients.characters[1], false).await?;
    let logout = client.session_mut().logout().await?;
    require_clean_logout("initial", logout)?;
    require_release_state(&mut database, &initial_id, clients.characters[1], true).await?;
    evidence("I09_POSTGRES_ASSERTION clean_logout_release=verified");
    let transcript = oteryn_dev_client::take_qualification_transcript()
        .ok_or("I09 original received frames unavailable")?;
    drop(client);
    let (mut relogged, relog_transcript) =
        oteryn_dev_client::with_qualification_transcript(async {
            let client = connect_world(clients, 0xca).await?;
            let transcript = oteryn_dev_client::take_qualification_transcript()
                .ok_or("I09 original relog frames unavailable")?;
            Ok::<_, Box<dyn std::error::Error>>((client, transcript))
        })
        .await?;
    if relogged.join_snapshot().game_session_id == session
        || relogged.world_spatial().actor_position != position
    {
        return Err("I09 relog did not open a new session at the saved position".into());
    }
    assert_chair(&mut relogged, target, item)?;
    let relog_id = uuid_text(relogged.join_snapshot().game_session_id.as_bytes());
    let cleanup = relogged.session_mut().logout().await?;
    require_clean_logout("relog", cleanup)?;
    drop(relogged);
    let (observer_id, observer_transcript) =
        oteryn_dev_client::with_qualification_transcript(async {
            if clients.accounts[0] == clients.accounts[1] || clients.characters[0] == clients.characters[1] {
                return Err("I09 public observer must use a distinct account and character".into());
            }
            let mut observer = connect_character(clients, 0xcb, 0).await?;
            let observer_id = uuid_text(observer.join_snapshot().game_session_id.as_bytes());
            if observer_id == initial_id || observer_id == relog_id {
                return Err("I09 public observer reused the rotating character's session".into());
            }
            require_release_state(&mut database, &observer_id, clients.characters[0], false).await?;
            let observer_start = tile(observer.world_spatial().actor_position)?;
            let path = route(world, observer_start, |cell| cell == approach)
                .ok_or_else(|| format!("I09 no route from {observer_start:?} to fixed public approach {approach:?}"))?;
            walk_once(&mut observer, world, &path).await?;
            require_actor_tile(tile(observer.world_spatial().actor_position)?, approach)?;
            assert_chair(&mut observer, target, item)?;
            let transcript = oteryn_dev_client::take_qualification_transcript()
                .ok_or("I09 original public observer frames unavailable")?;
            let logout = observer.session_mut().logout().await?;
            require_mechanic_acceptance(logout.status, "I09 observer cleanup logout")?;
            if logout.result != Some(LogoutResult::Accepted) {
                return Err("I09 observer cleanup logout refused".into());
            }
            Ok::<_, Box<dyn std::error::Error>>((observer_id, transcript))
        }).await?;
    let binding = format!(
        "{initial_id} {before_use_sequence} {} {} {} {} {} {relog_id} {observer_id}",
        use_command_id,
        use_reply_sequence,
        delta.server_sequence,
        logout.command_id,
        logout.result_server_sequence
    );
    capture(capture_root, "i09-original-frames.bin", &transcript)?;
    capture(capture_root, "i09-relog-frames.bin", &relog_transcript)?;
    capture_envelopes(capture_root, "i09-envelopes.json", &transcript, &initial_id)?;
    capture_envelopes(
        capture_root,
        "i09-relog-envelopes.json",
        &relog_transcript,
        &relog_id,
    )?;
    capture(capture_root, "i09-observer-frames.bin", &observer_transcript)?;
    capture_envelopes(capture_root, "i09-observer-envelopes.json", &observer_transcript, &observer_id)?;
    capture(capture_root, "i09-binding.txt", binding.as_bytes())?;
    capture(capture_root, "i09-map-before.bin", &before.payload)?;
    capture(
        capture_root,
        "i09-map-before.txt",
        format!(
            "{} {} {} {}\n",
            delta.server_sequence - 1,
            before.domain_id,
            before.snapshot_type,
            before.revision
        )
        .as_bytes(),
    )?;
    capture(
        capture_root,
        "i09-expected.txt",
        format!(
            "{} {} {} {} {} {} {} {} {} {} {} {} {} {}\n",
            delta.base_revision,
            delta.new_revision,
            delta.server_sequence,
            definition,
            item.appearance_id,
            target.x,
            target.y,
            target.floor,
            ordinal,
            stale.command_id,
            stale.server_sequence,
            position.x,
            position.y,
            position.floor
        )
        .as_bytes(),
    )?;
    evidence(&format!("I09_ORIGINAL_FRAME_BINDING {binding}"));
    Ok(())
}

fn assert_chair(
    client: &mut oteryn_dev_client::DevClientSession,
    target: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    expected: &world_map::MapItem,
) -> TestResult {
    let snapshot = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("I09 verification map absent")?;
    require_unique_chair(&snapshot.tiles, target, expected)
}

fn require_unique_chair(
    tiles: &[world_map::MapTile],
    target: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    expected: &world_map::MapItem,
) -> TestResult {
    let MapOrigin::BaseOrdinal { ordinal, .. } = expected.origin else {
        return Err("I09 expected chair has no placement ordinal".into());
    };
    let mut matching_tiles = tiles.iter().filter(|tile| tile.position == target);
    let tile = matching_tiles.next().ok_or("I09 chair tile absent")?;
    if tile.more || matching_tiles.next().is_some() || expected.count != 1 {
        return Err("I09 requires a complete tile stack and a single chair item".into());
    }
    let mut chairs = tile.items.iter()
        .filter(|item| matches!(item.appearance_id, 2358..=2361)
            || matches!(item.origin, MapOrigin::BaseOrdinal { ordinal: actual, .. } if actual == ordinal));
    if chairs.next() != Some(expected) || chairs.next().is_some() {
        return Err("I09 map must show exactly one chair at the expected placement and revision".into());
    }
    Ok(())
}

fn require_clean_logout(phase: &str, outcome: oteryn_dev_client::LogoutOutcome) -> TestResult {
    if outcome.status != CommandStatus::Accepted || outcome.result != Some(LogoutResult::Accepted) {
        return Err(format!("I09 {phase} first logout refused: {outcome:?}").into());
    }
    Ok(())
}

async fn require_release_state(
    database: &mut sqlx::PgConnection,
    session: &str,
    character: [u8; 16],
    terminal: bool,
) -> TestResult {
    let (state, transport, no_loss): (i16, bool, bool) = sqlx::query_as(
        "SELECT session_state, current_transport_ref IS NOT NULL,
                control_loss_epoch IS NULL AND original_grace_deadline IS NULL
         FROM game_durability_reconnect_sessions
         WHERE game_session_id=$1::text::uuid AND character_id=encode($2,'hex')::uuid",
    )
    .bind(session)
    .bind(character.as_slice())
    .fetch_one(database)
    .await?;
    check_release_state((state, transport, no_loss), terminal)
}

fn check_release_state(state: (i16, bool, bool), terminal: bool) -> TestResult {
    let expected = if terminal { (3, false, true) } else { (2, true, true) };
    if state != expected {
        return Err(format!("I09 session release expected {expected:?}, observed {state:?}").into());
    }
    Ok(())
}

fn require_actor_tile(actual: TilePos, expected: TilePos) -> TestResult {
    if actual != expected {
        return Err(format!("I09 route expected {expected:?}, observed {actual:?}").into());
    }
    Ok(())
}

async fn probe_chair_collision(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    approach: TilePos,
    chair: TilePos,
) -> TestResult {
    for (from, direction, to) in [(approach, StepDirection::West, chair), (chair, StepDirection::East, approach)] {
        require_actor_tile(tile(client.world_spatial().actor_position)?, from)?;
        if destination(world, from, direction) != Some(to) {
            return Err("I09 fixed chair collision probe has invalid landing".into());
        }
        walk_once(client, world, &[direction]).await?;
        require_actor_tile(tile(client.world_spatial().actor_position)?, to)?;
    }
    Ok(())
}

async fn walk_once(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    let mut from = tile(client.world_spatial().actor_position)?;
    for &direction in path {
        require_actor_tile(tile(client.world_spatial().actor_position)?, from)?;
        let expected =
            destination(world, from, direction).ok_or("I09 route step has no landing")?;
        let outcome = client.step(direction).await?;
        require_mechanic_acceptance(outcome.status, "I09 fixed route step")?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(format!(
                "I09 first step {direction:?} from {from:?} to {expected:?} failed: {outcome:?}"
            )
            .into());
        }
        // Honor the committed movement cadence before sending the next action.
        if outcome.step_wait_ms != 0 {
            client
                .service_liveness(Duration::from_millis(u64::from(outcome.step_wait_ms)))
                .await?;
        }
        require_actor_tile(tile(client.world_spatial().actor_position)?, expected)?;
        from = expected;
        client.take_events();
        verdict::Harness::global().progress("walk");
    }
    Ok(())
}

#[test]
fn i09_rejects_shifted_collision_probe_and_loss_release() {
    let chair = TilePos { x: 32355, y: 32206, floor: -7 };
    let approach = TilePos { x: 32356, ..chair };
    assert!(require_actor_tile(approach, chair).is_err());
    assert!(require_actor_tile(chair, chair).is_ok());
    assert!(check_release_state((2, true, true), true).is_err());
    assert!(check_release_state((3, false, false), true).is_err());
    assert!(check_release_state((3, false, true), true).is_ok());
}

#[test]
fn i09_logout_gate_keeps_refusal_details() {
    let accepted = oteryn_dev_client::LogoutOutcome {
        command_id: 60,
        result_server_sequence: 187,
        status: CommandStatus::Accepted,
        result: Some(LogoutResult::Accepted),
    };
    assert!(require_clean_logout("initial", accepted).is_ok());
    for (result, detail) in [
        (Some(LogoutResult::Busy), "Busy"),
        (Some(LogoutResult::InFight { retry_after_ms: 500 }), "InFight"),
        (None, "result: None"),
    ] {
        let refusal = oteryn_dev_client::LogoutOutcome {
            status: CommandStatus::Rejected,
            result,
            ..accepted
        };
        let error = require_clean_logout("initial", refusal).unwrap_err().to_string();
        assert!(error.contains(detail));
        assert!(error.contains("result_server_sequence: 187"));
    }
}
