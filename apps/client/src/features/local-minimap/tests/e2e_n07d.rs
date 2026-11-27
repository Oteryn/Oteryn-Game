//! Replay the two live admissions and their intervening logout, then verify private minimap memory.
use super::*;
use oteryn_platform_client::native_login::parse_uuid_v7;
use oteryn_protocol_oteryn::CommandStatus;
#[cfg(bevy_shell)]
use oteryn_protocol_oteryn::world_map::{
    STATE_DOMAIN_WORLD_MAP_VIEW, WorldMapViewSnapshot, decode_world_map_snapshot, in_window,
};
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
    world: &str,
    character: CharacterId,
) -> Result<(Session<DuplexStream>, crate::play::PlayView), Error> {
    let accepted_frame = read_frame(&mut envelope.payload.as_slice()).await?;
    let raw = decode_wire_envelope(&accepted_frame)?;
    let accepted = decode_server_accepted(raw.payload())?;
    if accepted.world_id.as_bytes() != &id(world)?
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
    let view = crate::native_entry::admitted_play_view(&session, character, world)?;
    assert_eq!(
        view.minimap_cache_identity(),
        Some((*character.as_bytes(), world.to_owned()))
    );
    Ok((session, view))
}

fn assert_remembered_cells(minimap: &Minimap, walkable: &BTreeSet<CellPosition>) {
    let remembered: BTreeSet<_> = minimap.cells.keys().copied().collect();
    assert_eq!(
        remembered, *walkable,
        "remembered minimap coordinates differ from captured walkable terrain"
    );
}

#[cfg(bevy_shell)]
fn pinned_digest(hex: &str) -> Result<[u8; 32], Error> {
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("N07d invalid pinned cache digest".into());
    }
    let mut digest = [0; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)?;
    }
    Ok(digest)
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
    let mut host = crate::bevy_shell::N07dHost::new(true)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(verify(
            [first, step, logout, second],
            terrain,
            ground,
            &expected,
            &mut host,
        ))
}

#[cfg(bevy_shell)]
async fn check_reply(receipt: &Envelope, session: &[u8; 16], command: u64) -> Result<(), Error> {
    let mut frames = receipt.payload.as_slice();
    while !frames.is_empty() {
        let bytes = read_frame(&mut frames).await?;
        let frame = decode_wire_envelope(&bytes)?;
        if frame.server_sequence() == receipt.server_sequence {
            assert_eq!(
                frame.message_type(),
                oteryn_protocol_oteryn::MessageType::CommandResult
            );
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
    [first, step, logout, second]: [Envelope; 4],
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
        assert_eq!(
            command.command_type,
            world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT
        );
        assert_eq!(
            world_spatial::decode_step_intent(command.payload)
                .map_err(|error| format!("{error:?}"))?,
            world_spatial::StepDirection::East
        );
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
    let (session, view) = admit(client, &first, &first_id, world_text, character).await?;
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
    let pinned_binding = (
        pinned_digest(value(expected, "content_generation")?)?,
        pinned_digest(value(expected, "bundle_digest")?)?,
        value(expected, "reset_epoch")?.parse()?,
    );
    assert_eq!(binding(&map), pinned_binding);
    assert_eq!(map.tiles.len().to_string(), value(expected, "tiles")?);

    let (link, commands, outbox) = crate::play::play_channel();
    host.admit(view, link);
    let pixels = host.minimap_pixels()?;
    let first_colors = assert_projected_ground(host.view()?, &map, &walkable, &pixels)?;
    {
        let minimap = super::lock(&host.view()?.minimap);
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
    let cache_path = super::lock(&host.view()?.minimap)
        .path()
        .ok_or("N07d missing live cache binding")?;
    assert!(
        !cache_path.exists(),
        "N07d expected fresh cache before periodic save"
    );
    let expected_first_disk: BTreeSet<_> = first_colors
        .iter()
        .map(|(&(floor, x, y), &color)| [i32::from(floor), x, y, i32::from(color)])
        .collect();
    // Let the ordinary host schedule write exploration before any marker or logout action.
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            host.poll();
            if super::lock(&host.view()?.minimap).save_task.is_none()
                && let Some(saved) = Saved::read_from(&cache_path)?
            {
                assert_eq!(
                    saved.cells.into_iter().collect::<BTreeSet<_>>(),
                    expected_first_disk
                );
                assert!(saved.markers.is_empty());
                return Ok::<_, Error>(());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .map_err(|_| "N07d periodic exploration save did not complete")??;
    host.view_mut()?.set_live_map(&map)?;
    assert_eq!(
        super::lock(&host.view()?.minimap).cells,
        first_colors,
        "N07d repeated snapshot changed exploration"
    );
    let marker = Marker {
        x: origin.x,
        y: origin.y,
        floor: origin.floor,
        label: "Mark".into(),
        color: 0,
    };
    assert!(super::lock(&host.view()?.minimap).markers.is_empty());
    host.route(create_mark_key()?);
    assert!(super::lock(&host.view()?.minimap).markers.is_empty());
    assert!(
        Saved::read_from(&cache_path)?
            .ok_or("N07d baseline save missing")?
            .markers
            .is_empty()
    );
    host.click_minimap_button("Save mark")?;
    assert_eq!(
        super::lock(&host.view()?.minimap).markers,
        std::slice::from_ref(&marker)
    );
    wait_for_saved_marks(host, &cache_path, std::slice::from_ref(&marker)).await?;
    let view = host.view()?;
    let worker = tokio::spawn(crate::play::run_session(session, commands, outbox));
    let (_, _, _, zoom, tracking) = view.minimap_camera();
    assert_eq!(
        view.minimap_camera(),
        (origin.x, origin.y, origin.floor, zoom, true)
    );
    assert!(tracking);
    host.step_east()?;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            host.poll();
            let view = host.view()?;
            if view.minimap_camera()
                == (destination.x, destination.y, destination.floor, zoom, true)
                && moved_ground
                    .iter()
                    .all(|point| super::lock(&view.minimap).cells.contains_key(point))
            {
                break;
            }
            tokio::task::yield_now().await;
        }
        Ok::<_, Error>(())
    })
    .await??;
    check_reply(&step, &first_id, first.command_id).await?;
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
    let view = host.view()?;
    let remembered = super::lock(&view.minimap).cells.clone();
    for key in &retained {
        assert_eq!(
            remembered.get(key),
            first_colors.get(key),
            "N07d movement changed retained colour at {key:?}"
        );
    }
    assert!(
        moved_ground.difference(&walkable).next().is_some(),
        "N07d needs newly explored cells"
    );
    let before_logout = std::fs::read(&cache_path)?;
    let before = Saved::read_from(&cache_path)?.ok_or("N07d baseline save missing")?;
    assert!(moved_ground.difference(&walkable).all(|&(floor, x, y)| !before.cells.iter().any(|cell| cell[..3] == [i32::from(floor), x, y])));
    // Keep the final cache owner alive so dropping the view cannot save it.
    let retained_cache = view.minimap.clone();
    let periodic_deadline = {
        let cache = super::lock(&retained_cache);
        assert!(cache.cells_dirty, "N07d movement needs a pending save at logout");
        assert!(cache.save_task.is_none(), "N07d movement was already scheduled for save");
        tokio::time::Instant::from_std(cache.saved_at + cache.retry_delay)
    };
    assert_remembered_cells(&super::lock(&view.minimap), &union);
    check_reply(&logout, &first_id, step.command_id + 1).await?;
    host.logout()?;
    tokio::time::timeout_at(periodic_deadline, async {
        while !host.logged_out() {
            host.poll();
            tokio::task::yield_now().await;
        }
    })
    .await?;
    worker.await?;
    first_peer.await??;
    tokio::time::timeout_at(periodic_deadline, async {
        while host.has_retired_saves() {
            host.poll();
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?;
    assert_ne!(
        std::fs::read(&cache_path)?,
        before_logout,
        "N07d logout did not flush newly explored terrain"
    );
    let saved = Saved::read_from(&cache_path)?.ok_or("N07d logout cache missing")?;
    let expected_disk: BTreeSet<_> = remembered
        .iter()
        .map(|(&(floor, x, y), &color)| [i32::from(floor), x, y, i32::from(color)])
        .collect();
    assert_eq!(
        saved.cells.into_iter().collect::<BTreeSet<_>>(),
        expected_disk
    );
    assert_eq!(saved.markers, std::slice::from_ref(&marker));

    assert!(tokio::time::Instant::now() < periodic_deadline, "N07d logout save reached the periodic deadline");
    drop(retained_cache);

    let (client, mut peer) = tokio::io::duplex(second.payload.len());
    let second_wire = second.payload.clone();
    let second_peer = tokio::spawn(async move {
        read_frame(&mut peer).await?;
        peer.write_all(&second_wire).await?;
        Ok::<_, Error>(())
    });
    let (relogged, reopened) = admit(client, &second, &second_id, world_text, character).await?;
    second_peer.await??;
    let relogged_map = relogged
        .world_map_snapshot()
        .ok_or("N07d missing relogged map")?;
    assert_eq!(binding(&relogged_map), pinned_binding);

    assert_eq!(relogged.world_spatial().actor_position, destination);
    assert_eq!(relogged_map.header.origin, destination);
    assert_eq!(relogged_map.header.first_visible_floor, destination.floor);
    assert_eq!(map_ground(&relogged_map), moved_ground);
    // Production join restores cells outside the new viewport. Applying the new
    // snapshot alone cannot create them, so they discriminate restoration from rebuild.
    host.admit(reopened, crate::play::play_channel().0);
    let pixels = host.minimap_pixels()?;
    assert_projected_ground(host.view()?, &relogged_map, &moved_ground, &pixels)?;
    {
        let minimap = super::lock(&host.view()?.minimap);
        assert_eq!(minimap.cells, remembered);
        assert_eq!(minimap.markers, std::slice::from_ref(&marker));
    }
    // The marker is one tile west of the actor. The glyph's left border covers
    // the edge of this tile, so sample its interior in the rendered image.
    let marker_tile = oteryn_renderer::TileCoord::new(
        15 + marker.x - destination.x,
        12 + marker.y - destination.y,
    );
    let [marker_x, marker_y] = oteryn_client_ui::minimap::view()?.tile_to_screen(marker_tile);
    let stride = (oteryn_client_ui::minimap::COLUMNS * oteryn_client_ui::minimap::TILE_PX) as usize;
    assert_eq!(
        pixels[(marker_y as usize + 2) * stride + marker_x as usize + 2],
        palette_rgba(180),
        "N07d restored mark must appear beneath the transparent part of the actor glyph"
    );
    check_markers(host, &cache_path, marker).await?;
    assert_eq!(super::lock(&host.view()?.minimap).cells, remembered);

    let (&(_, x, y), &color) = first_colors
        .iter()
        .find(|(key, _)| key.0 == destination.floor && retained.contains(key))
        .ok_or("N07d needs retained terrain on the actor floor")?;
    host.drag_minimap([x - destination.x, y - destination.y])?;
    assert_eq!(
        host.view()?.minimap_camera(),
        (x, y, destination.floor, zoom, false)
    );
    let grid = oteryn_client_ui::minimap::view()?;
    let center = grid.tile_to_screen(oteryn_renderer::TileCoord::new(15, 12));
    assert_sidebar_tile(&host.minimap_pixels()?, center, palette_rgba(color));
    host.click_minimap_button("Rose centre")?;
    assert_eq!(
        host.view()?.minimap_camera(),
        (destination.x, destination.y, destination.floor, zoom, true)
    );
    Ok(())
}

#[cfg(bevy_shell)]
fn assert_sidebar_tile(pixels: &[[u8; 4]], at: [f32; 2], rgba: [u8; 4]) {
    use oteryn_client_ui::minimap::{COLUMNS, TILE_PX};
    for y in at[1] as u32..at[1] as u32 + TILE_PX {
        for x in at[0] as u32..at[0] as u32 + TILE_PX {
            assert_eq!(
                pixels[(y * COLUMNS * TILE_PX + x) as usize],
                rgba,
                "N07d final sidebar pixel at ({x}, {y})"
            );
        }
    }
}

#[cfg(bevy_shell)]
#[should_panic(expected = "N07d final sidebar pixel")]
#[test]
fn n07d_rejects_a_later_overpaint_on_correct_terrain() {
    use oteryn_client_ui::minimap::{COLUMNS, ROWS, TILE_PX};
    let expected = palette_rgba(6);
    let mut pixels = vec![expected; (COLUMNS * ROWS * TILE_PX * TILE_PX) as usize];
    assert_sidebar_tile(&pixels, [0., 0.], expected);
    pixels[1] = palette_rgba(7);
    assert_sidebar_tile(&pixels, [0., 0.], expected);
}

#[cfg(bevy_shell)]
fn palette_rgba(color: u8) -> [u8; 4] {
    [
        (color / 36 % 6) * 51,
        (color / 6 % 6) * 51,
        (color % 6) * 51,
        255,
    ]
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
    pixels: &[[u8; 4]],
) -> Result<BTreeMap<CellPosition, u8>, Error> {
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
        (
            map.header.origin.x,
            map.header.origin.y,
            map.header.origin.floor,
            1,
            true
        )
    );
    let minimap = super::lock(&view.minimap);
    let scene = view.scene();
    let grid = oteryn_client_ui::minimap::view()?;
    let rendered: Vec<_> = crate::bevy_shell::n07d_sidebar_colors(scene).collect();
    let mut verified_pixels = 0;
    let mut expected_colors = BTreeMap::new();
    for tile in &map.tiles {
        let pos = tile.position;
        let key = (pos.floor, pos.x, pos.y);
        if !expected.contains(&key) {
            continue;
        }
        assert!(
            !tile.more,
            "N07d cannot infer a truncated terrain colour at {key:?}"
        );
        let mut color = 0;
        assert!(
            !tile.items.is_empty(),
            "N07d terrain has no appearance at {key:?}"
        );
        for item in &tile.items {
            let appearance = appearances
                .get(u32::from(item.appearance_id))
                .ok_or_else(|| {
                    format!(
                        "N07d missing pinned appearance {} at {key:?}",
                        item.appearance_id
                    )
                })?;
            if let Some(index) = appearance.minimap_color.filter(|index| *index != 0) {
                color = index;
            }
        }
        expected_colors.insert(key, color);
        assert_eq!(
            minimap.cells.get(&key),
            Some(&color),
            "N07d terrain colour at {key:?}"
        );
        let cell = oteryn_renderer::TileCoord::new(
            pos.x - map.header.origin.x + 15,
            pos.y - map.header.origin.y + 12,
        );
        if pos.floor == map.header.origin.floor && grid.contains(cell) {
            let at = grid.tile_to_screen(cell);
            let rgba = palette_rgba(color);
            let overlay = minimap
                .markers
                .iter()
                .find(|marker| (marker.floor, marker.x, marker.y) == key);
            let colors: Vec<_> = rendered
                .iter()
                .filter(|(position, _)| *position == at)
                .map(|(_, pixel)| *pixel)
                .collect();
            let mut expected = vec![rgba];
            if let Some(marker) = overlay {
                expected.push(palette_rgba([180, 5, 30, 210][usize::from(marker.color)]));
            }
            assert_eq!(colors, expected, "N07d sidebar paint order at {key:?}");
            let away_from_own_glyph =
                (pos.x - map.header.origin.x).abs() > 1 || (pos.y - map.header.origin.y).abs() > 1;
            if away_from_own_glyph && !view.entities.iter().any(|entity| entity.position == pos) {
                let rgba = expected[expected.len() - 1];
                assert_sidebar_tile(pixels, at, rgba);
                if at == [28., 24.] {
                    let origin = map.header.origin;
                    println!(
                        "N07d sidebar pixel (28,24) origin=({},{},{}) tile={key:?} rgba={rgba:?}",
                        origin.x, origin.y, origin.floor
                    );
                }
            }
            verified_pixels += 1;
        }
    }
    assert!(
        verified_pixels > 0,
        "N07d needs visible pinned terrain to verify"
    );
    Ok(expected_colors)
}

#[cfg(bevy_shell)]
fn create_mark_key() -> Result<oteryn_input_actions::NormalizedInputEvent, Error> {
    Ok(oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(16)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Alt)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    })
}

#[cfg(bevy_shell)]
async fn wait_for_saved_marks(
    host: &mut crate::bevy_shell::N07dHost,
    path: &Path,
    expected: &[Marker],
) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            host.poll();
            if super::lock(&host.view()?.minimap).save_task.is_none()
                && Saved::read_from(path)?.is_some_and(|saved| saved.markers == expected)
            {
                return Ok::<_, Error>(());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .map_err(|_| "N07d marks file did not reflect the UI action")?
}

#[cfg(bevy_shell)]
async fn check_markers(
    host: &mut crate::bevy_shell::N07dHost,
    path: &Path,
    mut marker: Marker,
) -> Result<(), Error> {
    let expected_terrain = super::lock(&host.view()?.minimap).cells.clone();
    let (x, y, floor, zoom, _) = host.view()?.minimap_camera();
    assert_eq!(floor, marker.floor);
    assert_eq!(zoom, 1);
    let cell = [15 + marker.x - x, 12 + marker.y - y];
    assert_eq!(
        super::lock(&host.view()?.minimap).position(cell[0], cell[1]),
        Some((marker.x, marker.y, marker.floor))
    );

    for (label, icon, color) in [
        ("Depot", "Green mark icon", 2),
        ("Temple", "Blue mark icon", 1),
    ] {
        host.open_mark_menu_at_cell(cell)?;
        host.click_minimap_button("Edit Mark")?;
        host.enter_mark_description(label)?;
        host.click_minimap_button(icon)?;
        host.click_minimap_button("Save mark")?;
        marker.label = label.into();
        marker.color = color;
        assert_eq!(
            super::lock(&host.view()?.minimap).markers,
            std::slice::from_ref(&marker)
        );
        wait_for_saved_marks(host, path, std::slice::from_ref(&marker)).await?;
        assert_saved_terrain(path, &expected_terrain)?;
    }

    host.open_mark_menu_at_cell(cell)?;
    host.click_minimap_button("Edit Mark")?;
    host.enter_mark_description("  ")?;
    host.click_minimap_button("Save mark")?;
    assert_eq!(
        super::lock(&host.view()?.minimap).markers,
        std::slice::from_ref(&marker)
    );
    assert_eq!(
        Saved::read_from(path)?
            .ok_or("N07d edited marks file missing")?
            .markers,
        [marker.clone()]
    );
    let before_invalid_floor = std::fs::read(path)?;
    {
        let mut minimap = super::lock(&host.view()?.minimap);
        let error = minimap.edit_marker(marker.x, marker.y, -5, marker.label.clone(), marker.color)
            .err().ok_or("N07d invalid marker floor must be refused")?;
        assert_eq!(error.kind(), std::io::ErrorKind::Other);
        assert_eq!(minimap.markers, std::slice::from_ref(&marker));
    }
    assert_eq!(std::fs::read(path)?, before_invalid_floor);
    host.click_minimap_button("Cancel mark")?;
    host.open_mark_menu_at_cell(cell)?;
    host.click_minimap_button("Edit Mark")?;
    host.enter_mark_description("")?;
    host.click_minimap_button("Save mark")?;
    marker.label.clear();
    assert_eq!(
        super::lock(&host.view()?.minimap).markers,
        std::slice::from_ref(&marker)
    );
    wait_for_saved_marks(host, path, std::slice::from_ref(&marker)).await?;
    assert_saved_terrain(path, &expected_terrain)?;

    host.open_mark_menu_at_cell(cell)?;
    host.click_minimap_button("Delete Mark")?;
    assert!(super::lock(&host.view()?.minimap).markers.is_empty());
    wait_for_saved_marks(host, path, &[]).await?;
    assert_saved_terrain(path, &expected_terrain)?;
    Ok(())
}

#[cfg(bevy_shell)]
fn assert_saved_terrain(path: &Path, expected: &BTreeMap<CellPosition, u8>) -> Result<(), Error> {
    let saved = Saved::read_from(path)?.ok_or("N07d edited cache missing")?;
    let expected: BTreeSet<_> = expected.iter().map(|(&(floor, x, y), &color)| {
        [i32::from(floor), x, y, i32::from(color)]
    }).collect();
    assert_eq!(saved.cells.len(), expected.len());
    assert_eq!(saved.cells.into_iter().collect::<BTreeSet<_>>(), expected);
    Ok(())
}

#[cfg(bevy_shell)]
fn replay_fixture_view(root: std::path::PathBuf) -> Result<crate::play::PlayView, Error> {
    use oteryn_session::{
        ActorPosition,
        world_map::{MapTile, MapViewHeader},
    };
    let origin = ActorPosition {
        x: 100,
        y: 200,
        floor: 7,
    };
    let mut view = crate::play::PlayView::new(
        oteryn_renderer::TileCoord::new(origin.x, origin.y),
        origin.floor,
        &[],
    )?;
    view.set_preference_character(id("01890a5d-ac96-774b-bcce-b302099a8058")?);
    view.bind_test_minimap(root);
    view.set_live_map(&WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
            origin,
            first_visible_floor: origin.floor,
        },
        tiles: vec![MapTile {
            position: ActorPosition {
                x: origin.x + 5,
                ..origin
            },
            ground_speed: 100,
            more: false,
            items: Vec::new(),
        }],
    })?;
    Ok(view)
}

#[cfg(bevy_shell)]
#[test]
fn atlas_resize_on_relog_rebinds_minimap_texture() -> Result<(), Error> {
    use oteryn_client_domain::world::CELL_PX;
    use oteryn_client_ui::minimap::{COLUMNS, TILE_PX};
    use oteryn_renderer::{
        lighting::PointLight,
        world::{ATLAS_COLUMNS, atlas_from_cells, builtin_rgba},
    };

    let root = tempfile::tempdir()?;
    let view = replay_fixture_view(root.path().to_owned())?;
    // Index 137 is in palette row 4. At the resized UV, the old atlas samples row 3.
    let color = 137;
    super::lock(&view.minimap).cells.insert((7, 105, 200), color);
    let mut host = crate::bevy_shell::N07dHost::new(true)?;
    host.admit(view, crate::play::play_channel().0);
    host.drag_minimap([13, 6])?;

    let at = [28., 24.];
    for smooth in [false, true] {
        for (rows, height) in [(3, 96), (4, 128)] {
            let mut cells = builtin_rgba();
            let count = ATLAS_COLUMNS * rows;
            cells.resize(count * (CELL_PX * CELL_PX * 4) as usize, 0);
            let atlas = atlas_from_cells(cells, count)?;
            assert_eq!((atlas.width(), atlas.height()), (4096, height));

            {
                let mut view = host.view_mut()?;
                view.scene.set_test_presentation_atlas(atlas)?;
                view.scene.set_lights(if smooth {
                    vec![PointLight {
                        position: at,
                        radius: 128.,
                        color: [96, 32, 32],
                    }]
                } else {
                    Vec::new()
                });
            }
            let pixel = host.minimap_pixels()?[(24 * COLUMNS * TILE_PX + 28) as usize];
            assert_eq!(
                pixel,
                palette_rgba(color),
                "sidebar pixel (28, 24) with a 4096x{height} atlas and smooth={smooth}"
            );
        }
    }
    Ok(())
}

#[cfg(bevy_shell)]
#[test]
fn minimap_render_target_readback_matches_scene() -> Result<(), Error> {
    let root = tempfile::tempdir()?;
    let mut host = crate::bevy_shell::N07dHost::new(true)?;
    host.admit(
        replay_fixture_view(root.path().to_owned())?,
        crate::play::play_channel().0,
    );
    let expected = palette_rgba(55);
    let at = [80., 48.];
    assert!(
        crate::bevy_shell::n07d_sidebar_colors(host.view()?.scene())
            .any(|(xy, color)| xy == at && color == expected)
    );
    assert_sidebar_tile(&host.minimap_pixels()?, at, expected);
    host.click_minimap_button("Rose E")?;
    let pixels = host.minimap_pixels()?;
    assert_sidebar_tile(&pixels, at, [0, 0, 0, 255]);
    // The actor glyph stays five tiles west of the new camera centre.
    assert_sidebar_tile(&pixels, [60., 48.], expected);
    Ok(())
}

#[cfg(bevy_shell)]
#[test]
fn sidebar_minimap_pixel_exact_on_both_lighting_paths() -> Result<(), Error> {
    use oteryn_renderer::lighting::PointLight;

    let root = tempfile::tempdir()?;
    let mut host = crate::bevy_shell::N07dHost::new(true)?;
    host.admit(
        replay_fixture_view(root.path().to_owned())?,
        crate::play::play_channel().0,
    );
    host.drag_minimap([13, 6])?;
    let at = [28., 24.];
    for smooth in [false, true] {
        for ambient in [[32, 64, 96], [255; 3]] {
            {
                let mut view = host.view_mut()?;
                view.scene.set_ambient(ambient)?;
                view.scene.set_lights(if smooth {
                    vec![PointLight {
                        position: at,
                        radius: 128.,
                        color: [96, 32, 32],
                    }]
                } else {
                    Vec::new()
                });
            }
            assert_sidebar_tile(&host.minimap_pixels()?, at, palette_rgba(55));
        }
    }
    // Weather tints the world atlas before either rendering path samples it.
    host.view_mut()?.scene.set_weather([0, 0, 255], 128);
    assert_sidebar_tile(&host.minimap_pixels()?, at, palette_rgba(55));
    Ok(())
}

#[cfg(bevy_shell)]
#[test]
fn ui_mark_edit_delete_pan_persist_on_disk() -> Result<(), Error> {
    let root = tempfile::tempdir()?;
    let mut host = crate::bevy_shell::N07dHost::new(false)?;
    let view = replay_fixture_view(root.path().to_owned())?;
    let path = super::lock(&view.minimap)
        .path()
        .ok_or("missing test cache path")?;
    host.admit(view, crate::play::play_channel().0);
    let (x, y, floor) = host.view()?.satellite_player();
    let marker = Marker {
        x,
        y,
        floor,
        label: "Mark".into(),
        color: 0,
    };
    host.route(create_mark_key()?);
    assert!(super::lock(&host.view()?.minimap).markers.is_empty());
    assert!(!path.exists());
    host.click_minimap_button("Cancel mark")?;
    assert!(super::lock(&host.view()?.minimap).markers.is_empty());
    assert!(!path.exists());
    host.route(create_mark_key()?);
    host.click_minimap_button("Save mark")?;
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?
        .block_on(async {
            wait_for_saved_marks(&mut host, &path, std::slice::from_ref(&marker)).await?;
            host.drag_minimap([5, 0])?;
            assert_eq!(host.view()?.minimap_camera(), (x + 5, y, floor, 1, false));
            check_markers(&mut host, &path, marker).await?;
            assert_eq!(host.view()?.satellite_player(), (x, y, floor));
            Ok(())
        })
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
