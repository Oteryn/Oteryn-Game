#[test]
fn srv7_actual_static_house_torch_transform_updates_native_pixels_and_use_revision()
-> Result<(), CakeBridgeError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(static_house_play_inner)?
        .join()
        .map_err(|_| "static House native play thread")?
}

fn static_house_play_inner() -> Result<(), CakeBridgeError> {
    use oteryn_client_runtime::world_interaction::{Action, Target};
    use oteryn_session::world_map::{MapOrigin, decode_world_map_delta, decode_world_map_snapshot};
    let capture: serde_json::Value = serde_json::from_str(include_str!("static-house-wire.json"))?;
    assert_eq!(capture["source_house_id"], 2844);
    assert_eq!(capture["house"], capture["viewer"]["house"]);
    let bytes = |key: &str| -> Result<Vec<u8>, CakeBridgeError> {
        let raw = capture[key].as_str().ok_or("actual House map bytes")?;
        if !raw.len().is_multiple_of(2) {
            return Err("odd House map bytes".into());
        }
        let value = raw
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| Ok(u8::from_str_radix(std::str::from_utf8(p)?, 16)?))
            .collect::<Result<Vec<_>, CakeBridgeError>>()?;
        assert_eq!(
            oteryn_client_assets::text_sha256(&value),
            capture["payload_sha256"][key]
                .as_str()
                .ok_or("sealed House map bytes")?
        );
        Ok(value)
    };
    let raw = bytes("world_map_snapshot_hex")?;
    let before = decode_world_map_snapshot(&raw).map_err(|e| format!("actual House map:{e:?}"))?;
    let delta = decode_world_map_delta(&before.header, &bytes("world_map_delta_hex")?)
        .map_err(|e| format!("actual House transform:{e:?}"))?;
    let viewer = &capture["viewer"]["position"];
    assert_eq!(
        (
            before.header.origin.x,
            before.header.origin.y,
            before.header.origin.floor
        ),
        (
            i32::try_from(viewer[0].as_i64().ok_or("viewer x")?)?,
            i32::try_from(viewer[1].as_i64().ok_or("viewer y")?)?,
            -i16::try_from(viewer[2].as_i64().ok_or("viewer floor")?)?
        )
    );
    let source = &capture["source_position"];
    let position = ActorPosition {
        x: i32::try_from(source[0].as_i64().ok_or("source x")?)?,
        y: i32::try_from(source[1].as_i64().ok_or("source y")?)?,
        floor: -i16::try_from(source[2].as_i64().ok_or("source floor")?)?,
    };
    let original = before
        .tiles
        .iter()
        .find(|tile| tile.position == position)
        .and_then(|tile| tile.items.iter().find(|item| item.appearance_id == 2929))
        .ok_or("actual2929 source BASE torch")?;
    let MapOrigin::BaseOrdinal {
        ordinal,
        object_revision: 0,
    } = original.origin
    else {
        return Err("actual original BASE revision0".into());
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.map_initialize(Some(raw));
    outbox.world_context_initialize(true, false);
    let mut native = view()?;
    native.set_live_map(&before)?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    native
        .local
        .set_world(Arc::new(World::resolve_live_map(&root, &before)?));
    native.rebuild()?;
    let pixels = static_house_tile_pixels(native.local.world().as_ref(), position)?;
    let old = native
        .world_context(TileCoord::new(position.x, position.y), &link)
        .ok_or("source BASE native context")?;
    assert!(matches!(&old.target,Target::Object(target) if target.expected_revision==0));
    // These are presentation reducer inputs, not newly encoded or replayed server frames.
    outbox.push(vec![SessionEvent::WorldMap(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: delta,
    })]);
    native
        .tick(&link)
        .map_err(|e| format!("native House map transform:{e:?}"))?;
    let after = crate::play::mailbox::lock(&outbox.0)
        .map
        .clone()
        .ok_or("actual transformed native map")?;
    let changed = after
        .tiles
        .iter()
        .find(|tile| tile.position == position)
        .and_then(|tile| tile.items.iter().find(|item| item.appearance_id == 2928))
        .ok_or("actual transformed2928")?;
    assert_eq!(
        changed.origin,
        MapOrigin::BaseOrdinal {
            ordinal,
            object_revision: 1
        }
    );
    native
        .local
        .set_world(Arc::new(World::resolve_live_map(&root, &after)?));
    native.rebuild()?;
    assert_ne!(
        static_house_tile_pixels(native.local.world().as_ref(), position)?,
        pixels,
        "source2929→2928 changes real native tile pixels"
    );
    assert!(
        !native.world_context_choose(&link, &old, Action::Use),
        "old revision0 menu is stale after actual server transform"
    );
    assert!(commands.try_recv().is_err());
    let current = native
        .world_context(TileCoord::new(position.x, position.y), &link)
        .ok_or("actual revision1 context")?;
    assert!(native.world_context_choose(&link, &current, Action::Use));
    assert!(
        matches!(
            commands.try_recv(),
            Ok(crate::combat_input::PlayCommand::WorldContext(command))
                if command.action == Action::Use
                    && matches!(&command.target, Target::Object(target) if target.expected_revision == 1)
                    && command.selection.as_ref().is_some_and(|selection| {
                        selection.position == position
                            && selection.binding == Some((
                                after.header.content_generation,
                                after.header.bundle_digest,
                                after.header.reset_epoch,
                            ))
                    })
        ),
        "next nativeUSE carries actual revision1 and exact current House map binding"
    );
    Ok::<(), CakeBridgeError>(())
}

type StaticHouseTilePixels = Vec<([i32; 2], Vec<u8>)>;

fn static_house_tile_pixels(
    world: &World,
    position: ActorPosition,
) -> Result<StaticHouseTilePixels, CakeBridgeError> {
    let tile = world
        .floor_tile(position.floor, position.x, position.y)
        .ok_or("actual static House tile pixels")?;
    let atlas = world.atlas();
    let width = usize::try_from(atlas.width())?;
    let columns = width / 32;
    let mut cells = Vec::new();
    for draw in tile.under.iter().chain(&tile.over) {
        let cell = usize::from(draw.cell);
        let (x, y) = ((cell % columns) * 32, (cell / columns) * 32);
        let mut bytes = Vec::new();
        for row in 0..32 {
            let start = ((y + row) * width + x) * 4;
            bytes.extend_from_slice(
                atlas
                    .rgba()
                    .get(start..start + 128)
                    .ok_or("actual native tile atlas bounds")?,
            );
        }
        cells.push((draw.offset, bytes));
    }
    assert!(!cells.is_empty());
    Ok(cells)
}
