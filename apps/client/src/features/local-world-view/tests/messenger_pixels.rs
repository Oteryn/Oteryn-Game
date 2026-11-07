#[test]
fn srv7_actual_messenger_source_palette_reaches_native_pixels() -> Result<(), HarePlayError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(messenger_native_pixels_thread)?
        .join()
        .map_err(|_| "actual NPC pixels thread")?
}
fn messenger_native_pixels_thread() -> Result<(), HarePlayError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(messenger_native_pixels_inner())
}
async fn messenger_native_pixels_inner() -> Result<(), HarePlayError> {
    use oteryn_session::{Admission, CharacterId, Look, Session};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let capture: serde_json::Value = serde_json::from_str(include_str!("messenger-wire.json"))?;
    let source = &capture["source"];
    assert_eq!(source["npc_key"], "oteryn:npc.messenger_of_santa");
    assert_eq!(
        source["map_fixture"],
        "existing grizzly caller Bundle; production source placement separately qualified by seasonal_population compiler"
    );
    let initial = hare_bytes(&capture, "initial_frames_hex", "initial_frames_sha256")?;
    let identity: [u8; 16] = hare_hex(
        source["actor_identity_hex"]
            .as_str()
            .ok_or("actual NPC identity")?,
    )?
    .try_into()
    .map_err(|_| "identitywidth")?;
    let character = CharacterId::decode(&hare_hex(
        source["character_id_hex"].as_str().ok_or("actual viewer")?,
    )?)?;
    let (client, mut server) = tokio::io::duplex(1 << 20);
    let (done, wait) = tokio::sync::oneshot::channel::<()>();
    let peer = tokio::spawn(async move {
        let n = usize::try_from(server.read_u32().await?)?;
        if n > 1 << 20 {
            return Err("bounded bootstrap".into());
        }
        let mut b = vec![0; n];
        server.read_exact(&mut b).await?;
        server.write_all(&initial).await?;
        let _ = wait.await;
        Ok::<(), HarePlayError>(())
    });
    let mut session = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: character,
            admission_material: b"srv7-actual-npc-read-only-play",
            client_build_id: "srv7-native-play",
            supported_capabilities: oteryn_session::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: std::time::Duration::from_secs(3),
        },
    )
    .await?;
    assert!(session.selected_capabilities().contains(&25));
    assert!(!session.selected_capabilities().contains(&104));
    let bytes = session
        .gated_snapshot(17)
        .ok_or("actual NPC caller map")?
        .payload
        .clone();
    let map = oteryn_session::world_map::decode_world_map_snapshot(&bytes)
        .map_err(|e| format!("actual caller map:{e:?}"))?;
    let at = &source["observer_runtime_cell"];
    assert_eq!(at[0].as_i64(), Some(i64::from(map.header.origin.x)));
    assert_eq!(at[1].as_i64(), Some(i64::from(map.header.origin.y)));
    assert_eq!(
        at[2].as_i64().map(|f| -f),
        Some(i64::from(map.header.origin.floor))
    );
    let own = *session
        .world_entities()
        .ok_or("actual NPC viewer identity")?
        .own_identity();
    let (link, _commands, outbox) = play_channel();
    outbox.properties_initialize(&session);
    outbox.appearance_initialize(&session);
    outbox.map_initialize(Some(bytes));
    let mut view = PlayView::from_join(session.join_snapshot())?;
    view.set_own_identity(own);
    view.set_live_map(&map)?;
    crate::play::session_stepper::publish(&mut session, &outbox, &mut Vec::new());
    view.tick(&link)
        .map_err(|e| format!("actual NPC native frame:{e:?}"))?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    view.local
        .set_world(std::sync::Arc::new(World::resolve_live_map(&root, &map)?));
    view.rebuild()?;
    let npc = view
        .appearances
        .iter()
        .find(|e| e.entity.entity.identity == identity)
        .ok_or("actual NPC reaches nativeScene")?;
    assert_eq!(
        npc.look,
        Some(Look::Outfit {
            look_type: 159,
            colours: [39, 99, 60, 117],
            addons: 0
        })
    );
    assert_eq!(
        Some(i64::from(npc.entity.position.x)),
        source["source_runtime_cell"][0].as_i64()
    );
    assert_eq!(
        Some(i64::from(npc.entity.position.y)),
        source["source_runtime_cell"][1].as_i64()
    );
    let palette = capture["palette_rgb"]
        .as_array()
        .ok_or("actual wire palette")?
        .iter()
        .map(|v| {
            Ok([
                u8::try_from(v[0].as_u64().ok_or("r")?)?,
                u8::try_from(v[1].as_u64().ok_or("g")?)?,
                u8::try_from(v[2].as_u64().ok_or("b")?)?,
            ])
        })
        .collect::<Result<Vec<_>, HarePlayError>>()?;
    let failures = SourceOutfitPixels::load()?.check(&view, identity, false, &palette)?;
    let _ = done.send(());
    peer.await??;
    assert!(
        failures.is_empty(),
        "actual Messenger palette pixels: {}",
        failures.join("; ")
    );
    Ok::<(), HarePlayError>(())
}
