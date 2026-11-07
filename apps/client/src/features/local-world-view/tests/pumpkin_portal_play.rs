type PumpkinPortalPlayError = Box<dyn std::error::Error + Send + Sync>;
fn pumpkin_portal_payload(
    capture: &serde_json::Value,
    key: &str,
) -> Result<Vec<u8>, PumpkinPortalPlayError> {
    use sha2::Digest;
    let raw = capture[key].as_str().ok_or("actual Pumpkin portal bytes")?;
    if !raw.len().is_multiple_of(2) {
        return Err("odd Pumpkin capture hex".into());
    }
    let bytes = raw
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect::<Result<Vec<_>, PumpkinPortalPlayError>>()?;
    let sha = sha2::Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(capture["payload_sha256"][key].as_str(), Some(sha.as_str()));
    Ok(bytes)
}
#[test]
fn srv7_pumpkin_settled_portal_capture_opens_visible_native_use_target()
-> Result<(), PumpkinPortalPlayError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(pumpkin_portal_native_inner)?
        .join()
        .map_err(|_| "native Pumpkin portal thread")?
}
fn pumpkin_portal_native_inner() -> Result<(), PumpkinPortalPlayError> {
    use oteryn_session::world_map::{decode_world_map_delta, decode_world_map_snapshot};
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("pumpkin-portal-wire.json"))?;
    assert_eq!(capture["schema"], "OTERYN_NATIVE_PUMPKIN_PORTAL_PLAY/v1");
    assert_eq!(capture["source"]["kind"], "PumpkinPortal");
    assert_eq!(capture["source"]["layout"], "DERIVED_NATIVE_ROOM");
    let closed =
        decode_world_map_snapshot(&pumpkin_portal_payload(&capture, "closed_snapshot_hex")?)
            .map_err(|e| format!("actual closed portal map: {e:?}"))?;
    let bundle = closed
        .header
        .bundle_digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        capture["source"]["bundle_digest_hex"].as_str(),
        Some(bundle.as_str()),
        "actual server map is bound to the independent source bundle"
    );
    let opened = decode_world_map_delta(
        &closed.header,
        &pumpkin_portal_payload(&capture, "open_delta_hex")?,
    )
    .map_err(|e| format!("actual open portal map: {e:?}"))?;
    let observer = &capture["source"]["observer_native"];
    assert_eq!(
        observer["x"].as_i64(),
        Some(i64::from(closed.header.origin.x))
    );
    assert_eq!(
        observer["y"].as_i64(),
        Some(i64::from(closed.header.origin.y))
    );
    assert_eq!(
        observer["floor"].as_i64(),
        Some(i64::from(closed.header.origin.floor))
    );
    let source = &capture["source"]["entry"];
    let portal = ActorPosition {
        x: i32::try_from(source[0].as_i64().ok_or("portal source x")?)?,
        y: i32::try_from(source[1].as_i64().ok_or("portal source y")?)?,
        floor: -i16::try_from(source[2].as_i64().ok_or("portal source floor")?)?,
    };
    assert!(
        closed
            .tiles
            .iter()
            .filter(|tile| tile.position == portal)
            .all(|tile| tile.items.iter().all(|item| item.appearance_id != 1949)),
        "the actual initial window hides the portal before the settled death job"
    );
    let (link, mut commands, outbox) = play_channel();
    outbox.world_context_initialize(true, true);
    let mut view = view()?;
    outbox.push(vec![SessionEvent::WorldMapSnapshot(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: closed,
    })]);
    let Some(PlayEvent::Pushed(before)) = link.poll() else {
        return Err("actual closed portal native push".into());
    };
    view.apply_pushed(&before)?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let world = World::resolve_live_map(
        &root,
        before.map.as_deref().ok_or("closed native portal map")?,
    )?;
    let pixels = world.atlas().rgba().to_vec();
    view.local.set_world(Arc::new(world));
    view.rebuild()?;
    outbox.push(vec![SessionEvent::WorldMap(AppliedDelta {
        server_sequence: 2,
        base_revision: 1,
        new_revision: 2,
        value: opened,
    })]);
    let Some(PlayEvent::Pushed(after)) = link.poll() else {
        return Err("actual opened portal native push".into());
    };
    let map = after.map.as_deref().ok_or("opened native portal map")?;
    assert!(
        map.tiles.iter().any(|tile| tile.position == portal
            && tile.items.iter().any(|item| item.appearance_id == 1949)),
        "settled world-job delta contains source1949 at its qualified entry cell"
    );
    view.apply_pushed(&after)?;
    let world = World::resolve_live_map(&root, map)?;
    assert_ne!(
        world.atlas().rgba(),
        pixels.as_slice(),
        "the actual portal adds its pinned native pixels"
    );
    view.local.set_world(Arc::new(world));
    view.rebuild()?;
    let menu = view
        .world_context(TileCoord::new(portal.x, portal.y), &link)
        .ok_or("actual opened portal native context target")?;
    assert!(
        view.world_context_choose(
            &link,
            &menu,
            oteryn_client_runtime::world_interaction::Action::Use
        ),
        "source portal is usable through native input"
    );
    assert!(
        matches!(
            commands.try_recv(),
            Ok(crate::combat_input::PlayCommand::WorldContext(_))
        ),
        "actual portal target produces the existing native USE command"
    );
    Ok::<(), PumpkinPortalPlayError>(())
}
