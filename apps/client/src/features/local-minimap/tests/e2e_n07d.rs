//! Replay the two live admissions and their intervening logout, then verify private minimap memory.
use super::*;
use oteryn_platform_client::native_login::parse_uuid_v7;
#[cfg(bevy_shell)]
use oteryn_protocol_oteryn::world_map::{
    STATE_DOMAIN_WORLD_MAP_VIEW, WorldMapViewSnapshot, decode_world_map_snapshot, in_window,
};
use oteryn_protocol_oteryn::CommandStatus;
#[cfg(bevy_shell)]
use oteryn_protocol_oteryn::{decode_server_accepted, decode_wire_envelope};
#[cfg(bevy_shell)]
use oteryn_session::{Admission, CharacterId, Session, read_frame};
use std::collections::BTreeSet;
#[cfg(bevy_shell)]
use std::time::Duration;
#[cfg(bevy_shell)]
use tokio::io::{AsyncWriteExt, DuplexStream};

#[cfg(bevy_shell)]
#[path = "e2e_n07d_tests.rs"]
mod replay_tests;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    command_id: u64,
    game_session_id: String,
    #[cfg_attr(not(bevy_shell), allow(dead_code))]
    payload: Vec<u8>,
    server_sequence: u64,
    status: u32,
}

impl Envelope {
    fn check(&self, session: &[u8; 16], command: u64, sequence: u64) -> Result<(), Error> {
        if parse_uuid_v7(&self.game_session_id).as_ref() != Some(session)
            || self.command_id != command
            || self.server_sequence != sequence
            || self.status != CommandStatus::Accepted as u32
        {
            return Err("N07d envelope session, command, sequence or status mismatch".into());
        }
        Ok(())
    }
}

#[cfg(bevy_shell)]
fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

fn id(value: &str) -> Result<[u8; 16], Error> {
    parse_uuid_v7(value).ok_or_else(|| "N07d capture identifier is not a UUIDv7".into())
}

#[cfg(bevy_shell)]
async fn admit(
    stream: DuplexStream,
    envelope: &Envelope,
    session_id: &[u8; 16],
    world: &[u8; 16],
    character: CharacterId,
) -> Result<Session<DuplexStream>, Error> {
    let accepted_frame = read_frame(&mut envelope.payload.as_slice()).await?;
    let raw = decode_wire_envelope(&accepted_frame)?;
    let accepted = decode_server_accepted(raw.payload())?;
    if accepted.world_id.as_bytes() != world
        || accepted.game_session_id.as_bytes() != session_id
        || accepted.next_command_id != envelope.command_id
    {
        return Err("N07d recorded ServerAccepted does not match the live admission".into());
    }
    let session = Session::admit(
        stream,
        Admission {
            appearance_items: None,
            schema_revision: accepted.schema_revision,
            character_id: character,
            admission_material: b"local-replay",
            client_build_id: "n07d-minimap-replay",
            supported_capabilities: oteryn_session::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(5),
        },
    )
    .await?;
    envelope.check(
        session.join_snapshot().game_session_id.as_bytes(),
        session.next_command_id(),
        session.last_server_sequence(),
    )?;
    Ok(session)
}

fn assert_remembered_cells(minimap: &Minimap, walkable: &BTreeSet<CellPosition>) {
    let remembered: BTreeSet<_> = minimap.cells.keys().copied().collect();
    assert_eq!(
        remembered, *walkable,
        "remembered minimap coordinates differ from captured walkable terrain"
    );
}

#[cfg(bevy_shell)]
fn binding(map: &WorldMapViewSnapshot) -> ([u8; 32], [u8; 32], u64) {
    (
        map.header.content_generation,
        map.header.bundle_digest,
        map.header.reset_epoch,
    )
}

#[cfg(bevy_shell)]
fn replay() -> Result<(), Error> {
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("N07d live capture directory is required")?;
    let root = std::path::Path::new(&root);
    let expected = std::fs::read_to_string(root.join("n07d-expected.txt"))?;
    if root.as_os_str() != value(&expected, "capture_dir")? {
        return Err("N07d capture belongs to a different run directory".into());
    }
    let first: Envelope =
        serde_json::from_slice(&std::fs::read(root.join("n07d-admission.json"))?)?;
    let step: Envelope = serde_json::from_slice(&std::fs::read(root.join("n07d-step.json"))?)?;
    let logout: Envelope = serde_json::from_slice(&std::fs::read(root.join("n07d-logout.json"))?)?;
    let second: Envelope = serde_json::from_slice(&std::fs::read(root.join("n07d-relog.json"))?)?;
    let terrain = std::fs::read(root.join("n07d-snapshot.bin"))?;
    let ground: (BTreeSet<CellPosition>, BTreeSet<CellPosition>) =
        serde_json::from_slice(&std::fs::read(root.join("n07d-ground.json"))?)?;
    // The production host shuts down its own runtime when dropped.
    let mut host = crate::bevy_shell::N07dHost::new()?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(verify(
            first, step, logout, second, terrain, ground, &expected, &mut host,
        ))
}

#[cfg(bevy_shell)]
async fn check_reply(receipt: &Envelope, session: &[u8; 16], command: u64) -> Result<(), Error> {
    let mut frames = receipt.payload.as_slice();
    while !frames.is_empty() {
        let bytes = read_frame(&mut frames).await?;
        let frame = decode_wire_envelope(&bytes)?;
        if frame.server_sequence() == receipt.server_sequence {
            assert_eq!(frame.message_type(), oteryn_protocol_oteryn::MessageType::CommandResult);
            let result = oteryn_protocol_oteryn::decode_command_result(frame.payload())?;
            assert_eq!(result.command_id, command);
            assert_eq!(result.status, CommandStatus::Accepted);
            return receipt.check(session, command, frame.server_sequence());
        }
    }
    Err("N07d recorded reply sequence missing".into())
}

#[cfg(bevy_shell)]
async fn verify(
    first: Envelope,
    step: Envelope,
    logout: Envelope,
    second: Envelope,
    terrain: Vec<u8>,
    ground: (BTreeSet<CellPosition>, BTreeSet<CellPosition>),
    expected: &str,
    host: &mut crate::bevy_shell::N07dHost,
) -> Result<(), Error> {
    let first_id = id(value(expected, "first_session")?)?;
    let second_id = id(value(expected, "second_session")?)?;
    if first_id == second_id {
        return Err("N07d relog needs a different game session".into());
    }
    let world_text = value(expected, "world")?;
    let world = id(world_text)?;
    let character = CharacterId::decode(&id(value(expected, "character")?)?)?;
    let (client, mut peer) = tokio::io::duplex(
        first
            .payload
            .len()
            .max(second.payload.len())
            .max(logout.payload.len())
            .max(step.payload.len()),
    );
    let first_wire = first.payload.clone();
    let step_wire = step.payload.clone();
    let logout_wire = logout.payload.clone();
    let step_command = step.command_id;
    let logout_command = logout.command_id;
    let first_peer = tokio::spawn(async move {
        use oteryn_protocol_oteryn::{logout, world_spatial};
        read_frame(&mut peer).await?;
        peer.write_all(&first_wire).await?;
        let bytes = read_frame(&mut peer).await?;
        let wire = decode_wire_envelope(&bytes)?;
        let generation = wire.connection_generation();
        let command = wire.client_command(generation)?;
        assert_eq!(command.command_id, step_command);
        assert_eq!(command.command_type, world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
        assert_eq!(world_spatial::decode_step_intent(command.payload).map_err(|error| format!("{error:?}"))?, world_spatial::StepDirection::East);
        peer.write_all(&step_wire).await?;
        let bytes = read_frame(&mut peer).await?;
        let wire = decode_wire_envelope(&bytes)?;
        let command = wire.client_command(generation)?;
        assert_eq!(command.command_id, logout_command);
        assert_eq!(command.command_type, logout::COMMAND_TYPE_LOGOUT_INTENT);
        logout::decode_logout_intent(command.payload).map_err(|error| format!("{error:?}"))?;
        peer.write_all(&logout_wire).await?;
        Ok::<_, Error>(())
    });
    let session = admit(client, &first, &first_id, &world, character).await?;
    let snapshot = session
        .gated_snapshot(STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("N07d missing admitted map")?;
    assert_eq!(snapshot.revision.to_string(), value(expected, "revision")?);
    assert_eq!(snapshot.payload, terrain);
    let expected_map = decode_world_map_snapshot(&terrain).map_err(|error| format!("{error:?}"))?;
    let (walkable, moved_ground) = ground;
    assert_eq!(map_ground(&expected_map), walkable);
    assert!(!walkable.is_empty());
    let map = session
        .world_map_snapshot()
        .ok_or("N07d malformed admitted map")?;
    let origin = map.header.origin;
    assert_eq!(
        format!("{},{},{}", origin.x, origin.y, origin.floor),
        value(expected, "origin")?
    );
    assert_eq!(
        map.header.first_visible_floor.to_string(),
        value(expected, "first_visible_floor")?
    );
    assert_eq!(
        map.header.reset_epoch.to_string(),
        value(expected, "reset_epoch")?
    );
    assert_eq!(map.tiles.len().to_string(), value(expected, "tiles")?);

    let open = |session: &Session<DuplexStream>| -> Result<crate::play::PlayView, Error> {
        let mut view = crate::play::PlayView::from_join(session.join_snapshot())?;
        view.bind_minimap_world(world_text);
        view.set_preference_character(*character.as_bytes());
        assert!(super::lock(&view.minimap).cells.is_empty());
        Ok(view.with_join_scene(session)?)
    };
    let (link, commands, outbox) = crate::play::play_channel();
    host.admit(open(&session)?, link);
    let view = host.view();
    assert_projected_ground(view, &map, &walkable)?;
    {
        let minimap = super::lock(&view.minimap);
        assert_eq!(
            minimap.cells.len().to_string(),
            value(expected, "walkable")?
        );
        assert_remembered_cells(&minimap, &walkable);
    }
    let destination = oteryn_session::ActorPosition {
        x: origin.x + 1,
        ..origin
    };
    let worker = tokio::spawn(crate::play::run_session(session, commands, outbox));
    let (_, _, _, zoom, tracking) = view.minimap_camera();
    assert_eq!(view.minimap_camera(), (origin.x, origin.y, origin.floor, zoom, true));
    assert!(tracking);
    host.step_east();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            host.poll();
            let view = host.view();
            if view.minimap_camera() == (destination.x, destination.y, destination.floor, zoom, true)
                && moved_ground.iter().all(|point| super::lock(&view.minimap).cells.contains_key(point))
            {
                break;
            }
            tokio::task::yield_now().await;
        }
        Ok::<_, Error>(())
    }).await??;
    check_reply(&step, &first_id, first.command_id).await?;
    let key = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(16)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Alt)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    };
    let marker = Marker {
        x: destination.x,
        y: destination.y,
        floor: destination.floor,
        label: "Mark".into(),
        color: 0,
    };
    assert!(super::lock(&host.view().minimap).markers.is_empty());
    host.route(key);
    let view = host.view();
    assert_eq!(super::lock(&view.minimap).markers, [marker.clone()]);
    let retained: BTreeSet<_> = walkable
        .iter()
        .copied()
        .filter(|&(floor, x, y)| {
            !in_window(destination, oteryn_session::ActorPosition { floor, x, y })
        })
        .collect();
    assert!(
        !retained.is_empty(),
        "N07d needs ground that the relog viewport cannot reconstruct"
    );
    let union: BTreeSet<_> = moved_ground.union(&retained).copied().collect();
    let remembered = super::lock(&view.minimap).cells.clone();
    let cache_path = super::lock(&view.minimap)
        .path()
        .ok_or("N07d missing live cache binding")?;
    assert_remembered_cells(&super::lock(&view.minimap), &union);
    check_reply(&logout, &first_id, step.command_id + 1).await?;
    host.logout();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !host.logged_out() {
            host.poll();
            tokio::task::yield_now().await;
        }
    }).await?;
    worker.await?;
    first_peer.await??;
    tokio::time::timeout(Duration::from_secs(5), async {
        while host.has_retired_saves() {
            host.poll();
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }).await?;
    assert!(
        cache_path.is_file(),
        "N07d live view exit did not save exploration"
    );

    let (client, mut peer) = tokio::io::duplex(second.payload.len());
    let second_wire = second.payload.clone();
    let second_peer = tokio::spawn(async move {
        read_frame(&mut peer).await?;
        peer.write_all(&second_wire).await?;
        Ok::<_, Error>(())
    });
    let relogged = admit(client, &second, &second_id, &world, character).await?;
    second_peer.await??;
    let relogged_map = relogged
        .world_map_snapshot()
        .ok_or("N07d missing relogged map")?;
    assert_eq!(binding(&map), binding(&relogged_map));

    assert_eq!(relogged.world_spatial().actor_position, destination);
    assert_eq!(relogged_map.header.origin, destination);
    assert_eq!(relogged_map.header.first_visible_floor, destination.floor);
    assert_eq!(map_ground(&relogged_map), moved_ground);
    // Production join restores cells outside the new viewport. Applying the new
    // snapshot alone cannot create them, so they discriminate restoration from rebuild.
    host.admit(open(&relogged)?, crate::play::play_channel().0);
    let reopened = host.view();
    assert_projected_ground(reopened, &relogged_map, &moved_ground)?;
    let minimap = super::lock(&reopened.minimap);
    assert_eq!(minimap.cells, remembered);
    assert_eq!(minimap.markers, [marker]);
    Ok(())
}

#[cfg(bevy_shell)]
fn map_ground(map: &WorldMapViewSnapshot) -> BTreeSet<CellPosition> {
    map.tiles
        .iter()
        .filter(|tile| {
            tile.ground_speed > 0
                && tile.position.floor >= map.header.first_visible_floor
                && in_window(map.header.origin, tile.position)
        })
        .map(|tile| (tile.position.floor, tile.position.x, tile.position.y))
        .collect()
}

#[cfg(bevy_shell)]
fn assert_projected_ground(
    view: &crate::play::PlayView,
    map: &WorldMapViewSnapshot,
    expected: &BTreeSet<CellPosition>,
) -> Result<(), Error> {
    let memory = &view.scene.minimap_memory;
    assert!(memory.binding.is_some());
    assert_eq!(memory, &view.local.preferences.minimap);
    let projected: BTreeSet<_> = memory.tiles.iter().copied().collect();
    let snapshot: BTreeSet<_> = map
        .tiles
        .iter()
        .map(|tile| {
            (
                tile.position.x,
                tile.position.y,
                tile.position.floor,
                tile.ground_speed,
            )
        })
        .collect();
    assert_eq!(projected, snapshot);
    use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, world};
    let root = std::path::PathBuf::from(
        std::env::var_os("OTERYN_ASSET_DIR").ok_or("N07d requires pinned appearance assets")?,
    );
    let store = AssetStore::open_pinned(
        &root.join("content/assets/files"),
        &root.join(world::MANIFEST_PATH),
        world::MANIFEST_SHA256,
    )?;
    let catalog = Catalog::load(&store)?;
    let appearances = AppearanceIndex::load(&store, &catalog)?;
    assert_eq!(
        view.minimap_camera(),
        (map.header.origin.x, map.header.origin.y, map.header.origin.floor, 1, true)
    );
    let minimap = super::lock(&view.minimap);
    let scene = view.scene();
    let grid = oteryn_client_ui::minimap::view()?;
    let rendered: Vec<_> = crate::bevy_shell::n07d_sidebar_colors(scene).collect();
    let mut verified_pixels = 0;
    for tile in &map.tiles {
        let pos = tile.position;
        let key = (pos.floor, pos.x, pos.y);
        if !expected.contains(&key) {
            continue;
        }
        assert!(!tile.more, "N07d cannot infer a truncated terrain colour at {key:?}");
        let mut color = 0;
        assert!(!tile.items.is_empty(), "N07d terrain has no appearance at {key:?}");
        for item in &tile.items {
            let appearance = appearances.get(u32::from(item.appearance_id))
                .ok_or_else(|| format!("N07d missing pinned appearance {} at {key:?}", item.appearance_id))?;
            if let Some(index) = appearance.minimap_color.filter(|index| *index != 0) {
                color = index;
            }
        }
        assert_eq!(minimap.cells.get(&key), Some(&color), "N07d terrain colour at {key:?}");
        let cell = oteryn_renderer::TileCoord::new(pos.x - map.header.origin.x + 15, pos.y - map.header.origin.y + 12);
        if pos.floor == map.header.origin.floor && grid.contains(cell) {
            let at = grid.tile_to_screen(cell);
            let rgba = [(color / 36 % 6) * 51, (color / 6 % 6) * 51, (color % 6) * 51, 255];
            assert!(
                rendered.contains(&(at, rgba)),
                "N07d missing rendered terrain pixel at {key:?} with RGBA {rgba:?}, observed {:?}", rendered.iter().filter(|(position, _)| *position == at).collect::<Vec<_>>()
            );
            verified_pixels += 1;
        }
    }
    assert!(verified_pixels > 0, "N07d needs visible pinned terrain to verify");
    Ok(())
}

#[test]
fn e2e_n07d_replays_admitted_map_into_the_explored_minimap() -> Result<(), Error> {
    #[cfg(bevy_shell)]
    return replay_tests::run();
    #[cfg(not(bevy_shell))]
    Err("N07d requires the production Bevy host".into())
}

#[test]
#[should_panic(expected = "remembered minimap coordinates differ from captured walkable terrain")]
fn e2e_n07d_rejects_wrong_coordinates_with_the_right_count() {
    let mut minimap = Minimap::default();
    minimap.cells.insert((7, 100, 200), 1);
    assert_remembered_cells(&minimap, &BTreeSet::from([(7, 99, 200)]));
}

#[test]
fn n07d_envelope_rejects_mismatched_session_command_and_sequence() -> Result<(), Error> {
    let session = "01934f10-7c00-7001-805b-3b1122334401";
    let other = "01934f10-7c00-7001-805b-3b1122334402";
    let envelope = Envelope {
        command_id: 1,
        game_session_id: session.into(),
        payload: Vec::new(),
        server_sequence: 2,
        status: CommandStatus::Accepted as u32,
    };
    envelope.check(&id(session)?, 1, 2)?;
    assert!(envelope.check(&id(other)?, 1, 2).is_err());
    assert!(envelope.check(&id(session)?, 2, 2).is_err());
    assert!(envelope.check(&id(session)?, 1, 3).is_err());
    Ok(())
}
