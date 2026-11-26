//! Replay the two live admissions and their intervening logout, then verify private minimap memory.
use super::*;
use oteryn_platform_client::native_login::parse_uuid_v7;
use oteryn_protocol_oteryn::world_map::{
    STATE_DOMAIN_WORLD_MAP_VIEW, WorldMapViewSnapshot, decode_world_map_snapshot, in_window,
};
use oteryn_protocol_oteryn::{CommandStatus, decode_server_accepted, decode_wire_envelope};
use oteryn_session::{Admission, CharacterId, Session, read_frame};
use std::collections::BTreeSet;
use std::time::Duration;
use tokio::io::{AsyncWriteExt, DuplexStream};

#[path = "e2e_n07d_tests.rs"]
mod replay_tests;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    command_id: u64,
    game_session_id: String,
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

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

fn id(value: &str) -> Result<[u8; 16], Error> {
    parse_uuid_v7(value).ok_or_else(|| "N07d capture identifier is not a UUIDv7".into())
}

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

fn binding(map: &WorldMapViewSnapshot) -> ([u8; 32], [u8; 32], u64) {
    (
        map.header.content_generation,
        map.header.bundle_digest,
        map.header.reset_epoch,
    )
}

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
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(verify(
            first, step, logout, second, terrain, ground, &expected,
        ))
}

async fn verify(
    first: Envelope,
    step: Envelope,
    logout: Envelope,
    second: Envelope,
    terrain: Vec<u8>,
    ground: (BTreeSet<CellPosition>, BTreeSet<CellPosition>),
    expected: &str,
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
    let first_peer = tokio::spawn(async move {
        read_frame(&mut peer).await?;
        peer.write_all(&first_wire).await?;
        read_frame(&mut peer).await?;
        peer.write_all(&step_wire).await?;
        read_frame(&mut peer).await?;
        peer.write_all(&logout_wire).await?;
        Ok::<_, Error>(())
    });
    let mut session = admit(client, &first, &first_id, &world, character).await?;
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

    let cache = tempfile::tempdir()?;
    let open = |session: &Session<DuplexStream>| -> Result<crate::play::PlayView, Error> {
        let mut view = crate::play::PlayView::from_join(session.join_snapshot())?;
        super::lock(&view.minimap).root = Some(cache.path().to_path_buf());
        view.bind_minimap_world(world_text);
        view.set_preference_character(*character.as_bytes());
        assert!(super::lock(&view.minimap).cells.is_empty());
        Ok(view.with_join_scene(session)?)
    };
    let mut view = open(&session)?;
    assert_projected_ground(&view, &map, &walkable);
    {
        let minimap = super::lock(&view.minimap);
        assert_eq!(
            minimap.cells.len().to_string(),
            value(expected, "walkable")?
        );
        assert_remembered_cells(&minimap, &walkable);
    }
    let initial = super::lock(&view.minimap).cells.clone();
    view.set_live_map(&map)?;
    assert_eq!(super::lock(&view.minimap).cells, initial);

    let movement = session
        .step(oteryn_protocol_oteryn::world_spatial::StepDirection::East)
        .await?;
    step.check(
        &first_id,
        movement.command_id,
        movement.result_server_sequence,
    )?;
    assert_eq!(movement.status, CommandStatus::Accepted);
    let destination = oteryn_session::ActorPosition {
        x: origin.x + 1,
        ..origin
    };
    assert_eq!(session.world_spatial().actor_position, destination);
    tokio::time::timeout(Duration::from_secs(5), async {
        while !session
            .world_map_snapshot()
            .is_some_and(|map| map.header.origin == destination)
        {
            session.wait_for_events(Duration::from_secs(5)).await?;
        }
        Ok::<_, oteryn_session::SessionError>(())
    })
    .await??;
    let moved = session
        .world_map_snapshot()
        .ok_or("N07d missing moved map")?;
    assert_eq!(moved.header.origin, destination);
    assert_eq!(map_ground(&moved), moved_ground);
    view.set_live_map(&moved)?;
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
    assert!(!cache_path.exists(), "N07d needs to exercise save on exit");
    assert_remembered_cells(&super::lock(&view.minimap), &union);
    let outcome = session.logout().await?;
    assert_eq!(outcome.status, CommandStatus::Accepted);
    assert_eq!(
        outcome.result,
        Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    );
    logout.check(
        &first_id,
        outcome.command_id,
        outcome.result_server_sequence,
    )?;
    first_peer.await??;
    drop(session);
    drop(view);
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
    let reopened = open(&relogged)?;
    assert_projected_ground(&reopened, &relogged_map, &moved_ground);
    let mut minimap = super::lock(&reopened.minimap);
    assert_eq!(minimap.cells, remembered);
    check_markers(&mut minimap, origin.x, origin.y, origin.floor)?;
    assert_eq!(minimap.cells, remembered);
    Ok(())
}

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

fn assert_projected_ground(
    view: &crate::play::PlayView,
    map: &WorldMapViewSnapshot,
    expected: &BTreeSet<CellPosition>,
) {
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
    let world = view.local.world();
    let minimap = super::lock(&view.minimap);
    for key in expected {
        assert_eq!(
            minimap.cells.get(key),
            Some(world.minimap_colors.get(key).unwrap_or(&55))
        );
    }
}

fn check_markers(minimap: &mut Minimap, x: i32, y: i32, floor: i16) -> Result<(), Error> {
    minimap.follow(x, y, floor);
    assert_eq!(minimap.position(15, 12), Some((x, y, floor)));

    // Markers are edited at that position; an invalid edit leaves them unchanged.
    minimap.edit_marker(x, y, floor, "Depot".into(), 2)?;
    let marker = Marker {
        x,
        y,
        floor,
        label: "Depot".into(),
        color: 2,
    };
    assert_eq!(minimap.markers.first(), Some(&marker));
    assert_eq!(minimap.markers.len(), 1);
    minimap.edit_marker(x, y, floor, "Temple".into(), 1)?;
    assert_eq!(
        minimap.markers,
        [Marker {
            label: "Temple".into(),
            color: 1,
            ..marker
        }]
    );
    let kept = minimap.markers.clone();
    assert!(
        minimap
            .edit_marker(x, y, -5, "Invalid floor".into(), 0)
            .is_err()
    );
    assert!(minimap.edit_marker(x, y, floor, "  ".into(), 0).is_err());
    assert_eq!(minimap.markers, kept);
    minimap.remove_marker(x, y, floor)?;
    assert!(minimap.markers.is_empty());
    Ok(())
}

#[test]
fn e2e_n07d_replays_admitted_map_into_the_explored_minimap() -> Result<(), Error> {
    replay_tests::run()
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
