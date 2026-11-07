#[test]
fn srv7_actual_hare_fifteen_forms_decode_and_show_source_pixels() -> Result<(), HarePlayError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(hare_fifteen_native_thread)?
        .join()
        .map_err(|_| "actual full15 source native thread")?
}
fn hare_fifteen_native_thread() -> Result<(), HarePlayError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(hare_fifteen_native_inner())
}
async fn hare_fifteen_native_inner() -> Result<(), HarePlayError> {
    use oteryn_session::{Admission, CharacterId, Look, Session};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("halloween-hare-wire.json"))?;
    assert_eq!(capture["complete"], true);
    let source = &capture["source"];
    let rows = capture["transformations"]
        .as_array()
        .ok_or("actual fifteen source forms")?;
    assert_eq!(rows.len(), 15);
    let actor: [u8; 16] = hare_hex(source["actor_hex"].as_str().ok_or("actual source player")?)?
        .try_into()
        .map_err(|_| "actual actor width")?;
    let character = CharacterId::decode(&hare_hex(
        source["character_id_hex"]
            .as_str()
            .ok_or("actual Character")?,
    )?)?;
    let initial = hare_bytes(&capture, "initial_frames_hex", "initial_sha256")?;
    let mut groups = rows
        .iter()
        .map(|row| hare_bytes(row, "frames_hex", "sha256"))
        .collect::<Result<Vec<_>, _>>()?;
    groups.push(hare_bytes(
        &capture,
        "cleanup_frames_hex",
        "cleanup_sha256",
    )?);
    let (client, mut server) = tokio::io::duplex(1 << 20);
    let (advance, mut next) = tokio::sync::mpsc::channel::<()>(1);
    let peer = tokio::spawn(async move {
        let size = usize::try_from(server.read_u32().await?)?;
        if size > 1 << 20 {
            return Err("bounded bootstrap".into());
        }
        let mut bootstrap = vec![0; size];
        server.read_exact(&mut bootstrap).await?;
        server.write_all(&initial).await?;
        for group in groups {
            next.recv().await.ok_or("real replay group")?;
            server.write_all(&group).await?;
        }
        next.recv().await.ok_or("replay completion")?;
        Ok::<(), HarePlayError>(())
    });
    let projection = oteryn_client_assets::seasonal_item_projection::seasonal_item_projection()
        .ok_or("sealed native appearanceItem projection")?;
    let mut session = Session::admit(
        client,
        Admission {
            appearance_items: Some(oteryn_session::AppearanceItemDictionary {
                content_generation: projection.content_generation,
                definition_refs: &projection.definition_refs,
            }),
            schema_revision: 1,
            character_id: character,
            admission_material: b"srv7-actual-fifteen-read-only-play",
            client_build_id: "srv7-native-play",
            supported_capabilities: oteryn_session::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: std::time::Duration::from_secs(3),
        },
    )
    .await?;
    assert!(session.selected_capabilities().contains(&25));
    assert!(!session.selected_capabilities().contains(&104));
    let map_bytes = session
        .gated_snapshot(17)
        .ok_or("actual source map")?
        .payload
        .clone();
    let mut map = oteryn_session::world_map::decode_world_map_snapshot(&map_bytes)
        .map_err(|e| format!("actual initialmap:{e:?}"))?;
    assert_eq!(
        source["observer_native"]["x"].as_i64(),
        Some(i64::from(map.header.origin.x))
    );
    assert_eq!(
        source["observer_native"]["y"].as_i64(),
        Some(i64::from(map.header.origin.y))
    );
    assert_eq!(
        source["observer_native"]["floor"].as_i64(),
        Some(i64::from(map.header.origin.floor))
    );
    let own_look = |s: &Session<tokio::io::DuplexStream>| {
        s.visible_appearances()
            .into_iter()
            .find(|e| e.entity.entity.identity == actor)
            .and_then(|e| e.look)
    };
    let baseline = own_look(&session).ok_or("actual original player look")?;
    let (link, _commands, outbox) = play_channel();
    outbox.properties_initialize(&session);
    outbox.appearance_initialize(&session);
    outbox.map_initialize(Some(map_bytes));
    let mut view = PlayView::from_join(session.join_snapshot())?;
    view.set_own_identity(actor);
    view.set_live_map(&map)?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut held = Vec::new();
    crate::play::session_stepper::publish(&mut session, &outbox, &mut held);
    view.tick(&link)
        .map_err(|e| format!("native initial:{e:?}"))?;
    view.local
        .set_world(std::sync::Arc::new(World::resolve_live_map(&root, &map)?));
    view.rebuild()?;
    let palette = capture["palette_rgb"]
        .as_array()
        .ok_or("actual wire palette")?
        .iter()
        .map(|v| {
            Ok([
                u8::try_from(v[0].as_u64().ok_or("red")?)?,
                u8::try_from(v[1].as_u64().ok_or("green")?)?,
                u8::try_from(v[2].as_u64().ok_or("blue")?)?,
            ])
        })
        .collect::<Result<Vec<_>, HarePlayError>>()?;
    let mut pixels = SourceOutfitPixels::load()?;
    let mut failures = Vec::new();
    let mut decoded = Vec::new();
    let _ = session.take_events();
    for row in rows {
        advance.send(()).await?;
        let target = row["last_server_sequence"]
            .as_u64()
            .ok_or("actual group finalsequence")?;
        while session.last_server_sequence() < target {
            let events = session
                .wait_for_events(std::time::Duration::from_secs(3))
                .await?;
            outbox.push(events);
        }
        map = session
            .world_map_snapshot()
            .ok_or("actual applied native map window")?;
        let look = own_look(&session).ok_or("actual transformed player")?;
        assert_eq!(
            format!("{look:?}"),
            row["look"].as_str().ok_or("actual source recorded look")?
        );
        decoded.push(look.clone());
        crate::play::session_stepper::publish(&mut session, &outbox, &mut held);
        view.tick(&link)
            .map_err(|e| format!("actual source form:{e:?}"))?;
        view.local
            .set_world(std::sync::Arc::new(World::resolve_live_map(&root, &map)?));
        view.rebuild()?;
        match look {
            Look::Item { definition_ref } => {
                assert_eq!(
                    definition_ref,
                    u32::try_from(
                        source["item_2096_definition_ref"]
                            .as_u64()
                            .ok_or("actual2096ref")?
                    )?
                );
                seasonal_item_official_pixels(&view, actor, 2096)?;
            }
            Look::Outfit { .. } => failures.extend(pixels.check(&view, actor, true, &palette)?),
            _ => return Err("source Hare only yields its15 closed forms".into()),
        }
    }
    assert_eq!(
        decoded
            .iter()
            .filter(|look| matches!(look, Look::Outfit { .. }))
            .count(),
        14
    );
    assert_eq!(
        decoded
            .iter()
            .filter(|look| matches!(look, Look::Item { .. }))
            .count(),
        1
    );
    advance.send(()).await?;
    let target = capture["cleanup_server_sequence"]
        .as_u64()
        .ok_or("actual expiry finalsequence")?;
    while session.last_server_sequence() < target {
        let events = session
            .wait_for_events(std::time::Duration::from_secs(3))
            .await?;
        outbox.push(events);
    }
    assert_eq!(
        own_look(&session),
        Some(baseline.clone()),
        "actual server expiry restores original appearance"
    );
    crate::play::session_stepper::publish(&mut session, &outbox, &mut held);
    view.tick(&link)
        .map_err(|e| format!("actual source expiry frame:{e:?}"))?;
    map = session.world_map_snapshot().ok_or("actual final map")?;
    view.local
        .set_world(std::sync::Arc::new(World::resolve_live_map(&root, &map)?));
    view.rebuild()?;
    assert_eq!(
        view.appearances
            .iter()
            .find(|entry| entry.entity.entity.identity == actor)
            .and_then(|entry| entry.look.clone()),
        Some(baseline.clone()),
        "native scene restores the actual base appearance after server expiry"
    );
    failures.extend(pixels.check(&view, actor, true, &palette)?);
    advance.send(()).await?;
    peer.await??;
    eprintln!(
        "SRV7_ACTUAL_FIFTEEN_DECODED item2096=true outfits14=true paced_steps={}",
        source["paced_steps"]
    );
    assert!(
        failures.is_empty(),
        "actual source outfit colour/addon pixel failures: {}",
        failures.join("; ")
    );
    Ok::<(), HarePlayError>(())
}
