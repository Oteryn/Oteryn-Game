#[test]
fn srv7_actual_cake_food_first_bite_and_ordinary_chat_reach_native_client()
-> Result<(), HarePlayError> {
    cake_food_native_thread(
        include_str!("cake-food-wire.json"),
        "cake-food-wire.json",
        true,
    )
}
#[test]
fn srv7_actual_cake_food_legacy_chat7_say_reaches_native_client() -> Result<(), HarePlayError> {
    cake_food_native_thread(
        include_str!("cake-food-legacy-wire.json"),
        "cake-food-legacy-wire.json",
        false,
    )
}
fn cake_food_native_thread(
    captured: &'static str,
    fixture: &'static str,
    scripted: bool,
) -> Result<(), HarePlayError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(cake_food_native_inner(captured, fixture, scripted))
        })?
        .join()
        .map_err(|_| "actual Cake Food native play thread")?
}
async fn cake_food_native_inner(
    captured: &'static str,
    fixture: &'static str,
    scripted: bool,
) -> Result<(), HarePlayError> {
    use oteryn_client_runtime::world_interaction::{Action, Target, world_object};
    use oteryn_session::{Admission, CharacterId, Session};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let capture: serde_json::Value = serde_json::from_str(captured)?;
    let source = &capture["source"];
    assert_eq!(source["chat7"], true);
    assert_eq!(source["ambient288"], false);
    assert_eq!(source["scripted386"], scripted);
    let transcript = hare_bytes(&capture, "frames_hex", "frames_sha256")?;
    let initial = hare_bytes(&capture, "initial_frames_hex", "initial_frames_sha256")?;
    let reply = hare_bytes(&capture, "reply_frames_hex", "reply_frames_sha256")?;
    assert_eq!([initial.as_slice(), reply.as_slice()].concat(), transcript);
    let payload = hare_bytes(source, "command_payload_hex", "command_payload_sha256")?;
    let expected = world_object::decode_use_intent(&payload)
        .map_err(|e| format!("actual Cake source USE:{e:?}"))?;
    let character = CharacterId::decode(&hare_hex(
        source["character_id_hex"]
            .as_str()
            .ok_or("actual source character")?,
    )?)?;
    let (client, mut peer) = tokio::io::duplex(1 << 20);
    let (done, wait) = tokio::sync::oneshot::channel::<()>();
    let replay = tokio::spawn(async move {
        let size = usize::try_from(peer.read_u32().await?)?;
        if size > 1 << 20 {
            return Err("bounded original native bootstrap".into());
        }
        let mut bootstrap = vec![0; size];
        peer.read_exact(&mut bootstrap).await?;
        peer.write_all(&initial).await?;
        eprintln!("SRV7_FOOD_REPLAY initial_sent");
        // Real native runtime drains liveness before sending USE. The
        // original server result becomes readable only after that USE.
        let command_size = usize::try_from(peer.read_u32().await?)?;
        if command_size > 1 << 20 {
            return Err("bounded actual USE frame".into());
        }
        let mut actual_command = vec![0; command_size];
        peer.read_exact(&mut actual_command).await?;
        eprintln!("SRV7_FOOD_REPLAY native_command_read bytes={command_size}");
        // Reuse the repository's read-only capture field oracle; do not
        // add a second protocol decoder or dependency to the native app.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let verified = std::process::Command::new("python3")
            .arg(root.join("tools/qualification/capture_cake_food_fixture.py"))
            .arg(
                root.join("apps/client/src/features/local-world-view/tests")
                    .join(fixture),
            )
            .arg("--verify-native-request")
            .arg(
                actual_command
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>(),
            )
            .output()
            .map_err(|error| {
                eprintln!("SRV7_FOOD_REPLAY verifier_spawn_error={error}");
                error
            })?;
        eprintln!(
            "SRV7_FOOD_REPLAY verifier_status={} stdout={} stderr={}",
            verified.status,
            String::from_utf8_lossy(&verified.stdout),
            String::from_utf8_lossy(&verified.stderr)
        );
        assert!(
            verified.status.success(),
            "actual native USE id/type/payload: {}",
            String::from_utf8_lossy(&verified.stderr)
        );

        peer.write_all(&reply).await?;
        let _ = wait.await;
        Ok::<(), HarePlayError>(())
    });
    let mut session = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: character,
            admission_material: b"srv7-actual-food-native-play",
            client_build_id: "srv7-native-play",
            supported_capabilities: oteryn_session::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: std::time::Duration::from_secs(3),
        },
    )
    .await?;
    assert!(session.selected_capabilities().contains(&7));
    assert!(!session.selected_capabilities().contains(&288));
    assert_eq!(session.selected_capabilities().contains(&386), scripted);
    let map = session
        .world_map_snapshot()
        .ok_or("actual source Cake map")?;
    let observer = &source["observer_runtime_cell"];
    assert_eq!(
        (
            map.header.origin.x,
            map.header.origin.y,
            map.header.origin.floor
        ),
        (
            i32::try_from(observer[0].as_i64().ok_or("actual viewer x")?)?,
            i32::try_from(observer[1].as_i64().ok_or("actual viewer y")?)?,
            -i16::try_from(observer[2].as_i64().ok_or("actual viewer floor")?)?
        )
    );
    let (link, mut commands, outbox) = play_channel();
    outbox.chat_initialize(true);
    outbox.properties_initialize(&session);
    outbox.map_initialize(session.gated_snapshot(17).map(|row| row.payload.clone()));
    outbox.appearance_initialize(&session);
    let mut native = PlayView::from_join(session.join_snapshot())?;
    if let Some(entities) = session.world_entities() {
        native.set_own_identity(*entities.own_identity());
    }
    native.set_live_map(&map)?;
    let mut held = Vec::new();
    crate::play::session_stepper::publish(&mut session, &outbox, &mut held);
    native
        .tick(&link)
        .map_err(|e| format!("actual Cake initial frame:{e:?}"))?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    native
        .local
        .set_world(std::sync::Arc::new(crate::map::World::resolve_live_map(
            &root, &map,
        )?));
    native.rebuild()?;
    let menu = map
        .tiles
        .iter()
        .filter_map(|tile| {
            native.world_context(
                oteryn_renderer::TileCoord::new(tile.position.x, tile.position.y),
                &link,
            )
        })
        .find(|menu| menu.target == Target::Object(expected.clone()))
        .ok_or("actual Cake source tile is a native usable context target")?;
    assert!(native.world_context_choose(&link, &menu, Action::Use));
    let command = match commands.try_recv()? {
        crate::combat_input::PlayCommand::WorldContext(command) => command,
        _ => return Err("native Cake USE command".into()),
    };
    assert_eq!(command.action, Action::Use);
    assert_eq!(command.target, Target::Object(expected.clone()));
    assert_eq!(
        world_object::encode_use_intent(&expected)
            .map_err(|e| format!("source USE encoding:{e:?}"))?,
        payload
    );
    eprintln!(
        "SRV7_FOOD_NATIVE before_run next={} sequence={}",
        session.next_command_id(),
        session.last_server_sequence()
    );
    if let Err(error) = outbox.world_context_run(&mut session, &command).await {
        let _ = done.send(());
        let peer = tokio::time::timeout(std::time::Duration::from_millis(250), replay).await;
        return Err(format!("actual Food world_context_run: {error:?}; peer={peer:?}").into());
    }
    eprintln!(
        "SRV7_FOOD_NATIVE after_run next={} sequence={} feedback={:?}",
        session.next_command_id(),
        session.last_server_sequence(),
        link.world_context_feedback()
    );
    let last = capture["last_server_sequence"]
        .as_u64()
        .ok_or("actual last captured sequence")?;
    while session.last_server_sequence() < last {
        outbox.push(
                            session
                                .wait_for_events(std::time::Duration::from_secs(3))
                                .await.map_err(|error|format!("actual Food pushed events: {error:?}; sequence={} target={last}; feedback={:?}; peer_finished={}",session.last_server_sequence(),link.world_context_feedback(),replay.is_finished()))?,
                        );
    }
    crate::play::session_stepper::publish(&mut session, &outbox, &mut held);
    native
        .tick(&link)
        .map_err(|e| format!("actual Cake committed native frame:{e:?}"))?;
    assert!(
        oteryn_client_ui::chat::panel_text(&link.chat_panel()).contains("Mmmm!"),
        "actual ordinary chat7 first-bite speech must render without ambient288"
    );
    assert!(
        link.world_context_feedback()
            .as_ref()
            .is_some_and(|text| text.contains("completed")),
        "native actualUSE result must confirm the first bite"
    );
    let expected_mode = if scripted {
        oteryn_session::ChatSpeechMode::MonsterSay
    } else {
        oteryn_session::ChatSpeechMode::Say
    };
    assert!(session.chat_log().is_some_and(|log|log.lines().any(|line|matches!(line,
                        oteryn_session::ChatLine::Local {mode,text,..} if *mode==expected_mode && text=="Mmmm!"
                    ))), "actual decoded speech mode must match negotiated scripted/legacy capability");
    assert_eq!(capture["mmm_count"], 1);
    let _ = done.send(());
    replay.await??;
    Ok::<(), HarePlayError>(())
}
