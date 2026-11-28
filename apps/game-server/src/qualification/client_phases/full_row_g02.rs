//! G02: named Wheel builds and code exchange (apps/client 0148-wheel-allocation presets).
//! 1. Admit the node-boot character and refresh the native Wheel panel from the server.
//! 2. Propose a build in the panel, export its `OW1:` code, store it in a named preset, save the
//!    preset book to disk and load it back (client-local exchange), then reset the proposal and
//!    import the code from the reloaded preset.
//! 3. Press Save Wheel: the server must confirm the imported points with a revision advance.
//! 4. Log out, freshly admit, refresh: the durable allocation must equal the imported build.
//! 5. Reset the Wheel through the panel and confirm the zero allocation (cleanup, same route).
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::wheel_allocation;
use oteryn_dev_client::client_ui::wheel_allocation::{Intent, WheelPanel, apply_result, request};
use oteryn_protocol_oteryn::atelier::v2::CAPABILITY_GEM_ATELIER_V2;
use oteryn_protocol_oteryn::wheel::CAPABILITY_WHEEL_ALLOCATION_V1;

const CORE_SLOT: usize = 20;
const POINTS: u16 = 10;
const LEVEL: u32 = 60;
const CASTLE_DOOR: TilePos = TilePos {
    x: 32314,
    y: 32180,
    floor: -7,
};
const CASTLE_APPROACH: TilePos = TilePos {
    y: 32181,
    ..CASTLE_DOOR
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let (status, detail) = match verdict::observe(journey(clients)).await {
        Ok(()) => ("PASS", "named-build-code-saved-and-durable".to_owned()),
        Err(error) => ("FAIL", token(&error.to_string())),
    };
    evidence(&format!(
        "e2e_all scenario=G02.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

/// Send one panel intent through the real session and apply the server answer to the panel.
async fn press(
    client: &mut DevClientSession,
    panel: &mut WheelPanel,
    intent: Intent,
    lane: u8,
    step: &str,
) -> TestResult {
    let command = request(panel, intent, v7(0x92, lane))
        .ok_or_else(|| format!("step={step} observed=panel-refused:{:?}", panel.feedback))?;
    let response = wheel_allocation::send(client.session_mut(), &command).await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("step={step} observed=status:{:?}", response.status).into());
    }
    if !apply_result(panel, &response) {
        return Err(format!("step={step} observed=view-not-applied:{:?}", panel.feedback).into());
    }
    Ok(())
}

fn admitted_panel(client: &DevClientSession) -> WheelPanel {
    let capabilities = client.selected_capabilities();
    let selected = capabilities.contains(&CAPABILITY_WHEEL_ALLOCATION_V1);
    WheelPanel {
        selected,
        atelier_v2: selected && capabilities.contains(&CAPABILITY_GEM_ATELIER_V2),
        ..WheelPanel::default()
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={reply:?}").into());
    }
    Ok(())
}

async fn paced_step(
    client: &mut DevClientSession,
    direction: StepDirection,
    expected: TilePos,
) -> TestResult {
    let reply = client.step(direction).await?;
    let actual = tile(client.world_spatial().actor_position)?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition != StepDisposition::Moved
        || reply.world_spatial_delta.is_none()
        || actual != expected
    {
        return Err(format!(
            "step=promotion-walk expected={expected:?} actual={actual:?} reply={reply:?}"
        )
        .into());
    }
    verdict::Harness::global().progress("G02 promotion walk");
    if reply.step_wait_ms > 0 {
        client
            .service_liveness(Duration::from_millis(u64::from(reply.step_wait_ms)))
            .await?;
    }
    client.take_events();
    Ok(())
}

async fn walk_to(
    client: &mut DevClientSession,
    world: &BundleWorld,
    goal: impl Fn(TilePos) -> bool,
) -> TestResult {
    let mut at = tile(client.world_spatial().actor_position)?;
    let path = route(world, at, goal).ok_or("step=promotion-walk observed=no-authored-path")?;
    for direction in path {
        at = destination(world, at, direction).ok_or("step=promotion-walk observed=no-landing")?;
        paced_step(client, direction, at).await?;
    }
    Ok(())
}

async fn promotion_line(
    client: &mut DevClientSession,
    npc: oteryn_dev_client::EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    use oteryn_protocol_oteryn::npc_service as wire;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("step=promotion-{text} encode={error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = wire::decode_npc_intent_result(&response.payload);
    if response.status != CommandStatus::Accepted
        || disposition != Ok(wire::NpcIntentDisposition::Ok)
    {
        return Err(format!(
            "step=promotion-{text} status={:?} disposition={disposition:?}",
            response.status
        )
        .into());
    }
    loop {
        let delta = client
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        if delta.server_sequence <= response.server_sequence {
            continue;
        }
        let conversation = wire::decode_npc_conversation(&delta.payload)
            .map_err(|error| format!("step=promotion-{text} conversation={error:?}"))?;
        if conversation.open.map(|(actor, _)| actor) != Some(npc) {
            return Err(format!("step=promotion-{text} observed=wrong-npc-conversation").into());
        }
        return Ok(conversation.lines);
    }
}

async fn promote(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    use crate::map::door::AuthoredClosedDoor;
    use oteryn_protocol_oteryn::world_object::UseDisposition;

    let key = crate::map::view::placement_key(CASTLE_DOOR, 1)
        .ok_or("step=castle-door observed=no-placement-key")?;
    if world.facts().doors().authored_closed(key) != Some(AuthoredClosedDoor::KeyUnlocked) {
        return Err("step=castle-door observed=wrong-authored-door".into());
    }
    walk_to(client, world, |at| at == CASTLE_APPROACH).await?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let reply = client.use_object(&placement, 0).await?;
    if reply.status != CommandStatus::Accepted || reply.disposition != UseDisposition::Committed {
        return Err(format!("step=castle-door observed={reply:?}").into());
    }
    paced_step(client, StepDirection::North, CASTLE_DOOR).await?;

    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let king = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.king_tibianus")
        .ok_or("step=promotion observed=no-authored-king")?;
    let [place] = king.placements.as_slice() else {
        return Err("step=promotion observed=ambiguous-king-placement".into());
    };
    walk_to(client, world, |at| {
        at.floor == place.floor && at.x.abs_diff(place.x).max(at.y.abs_diff(place.y)) <= 3
    })
    .await?;
    let basis = spatial_basis::current(client)?;
    let position = ActorPosition {
        x: i32::from(place.x),
        y: i32::from(place.y),
        floor: basis.floor(place.floor),
    };
    let npc = client
        .world_entities()
        .and_then(|view| {
            view.iter().find(|actor| {
                actor.kind == oteryn_dev_client::EntityKind::Npc && actor.position == position
            })
        })
        .map(|actor| actor.entity)
        .ok_or("step=promotion observed=no-king-at-authored-placement")?;
    if promotion_line(client, npc, "hi").await?.is_empty()
        || promotion_line(client, npc, "promotion").await?
            != ["Do you want to be promoted for 20000 gold coins?"]
        || promotion_line(client, npc, "yes").await? != ["You are now promoted."]
    {
        return Err("step=prepare observed=promotion-dialogue-did-not-confirm".into());
    }
    Ok(())
}

async fn prepare(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    character_prep::declare_seeded(
        "G02",
        "level-60-sorcerer,Premium,promotion-fee",
        "ordinary leveling is outside the named Wheel build journey budget",
    );
    let mut client = connect_character(clients, 0xdf, 0).await?;
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "g02-wheel".into(),
    )
    .await?;
    character_prep::grant_item(&client, 3043, 2).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(clients, 0xe0, 0).await?;
    premium_input(&client).await?;
    let vitals = client
        .actor_vitals()
        .ok_or("step=prepare observed=no-vitals")?;
    let (health, mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer",
        LEVEL,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("step=prepare observed=no-level-60-vital-maxima")?;
    if (vitals.max_health, vitals.max_mana) != (health, mana) {
        return Err(format!(
            "step=prepare observed={vitals:?};needed=max-health:{health},max-mana:{mana}"
        )
        .into());
    }
    evidence(&format!(
        "G02_CHECK step=derived-vitals level={LEVEL} max_health={health} max_mana={mana}"
    ));
    let world = full_e2ef7::qualification_world(clients)?;
    promote(&mut client, world).await?;
    logout(&mut client).await?;
    drop(client);
    connect_character(clients, 0xe1, 0).await
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    // The castle approach uses every server-issued step delay. Keep its time outside
    // the original 120-second Wheel verification budget.
    let (identity, client) = verdict::indexed_timeout(Duration::from_secs(300), async {
        let identity =
            character_prep::dedicated_character(clients, "NODE_BOOT_G02_ACCOUNT").await?;
        let client = prepare(&identity.clients(clients)).await?;
        TestResult::Ok((identity, client))
    })
    .await
    .map_err(|error| verdict::context("step=prepare observed=timeout".into(), error))??;
    verdict::indexed_timeout(
        Duration::from_secs(120),
        verify(&identity.clients(clients), client),
    )
    .await
    .map_err(|error| verdict::context("step=journey observed=timeout".into(), error))?
}

async fn verify(clients: &SeamClients<'_>, mut client: DevClientSession) -> TestResult {
    premium_input(&client).await?;
    let mut panel = admitted_panel(&client);
    if !panel.selected {
        return Err("step=admit observed=wheel-capability-not-selected".into());
    }
    press(&mut client, &mut panel, Intent::Refresh, 1, "refresh").await?;
    let initial = panel
        .confirmed_wheel
        .clone()
        .ok_or("step=refresh observed=no-confirmed-wheel")?;
    if initial.points != [0; 36] {
        return Err(format!(
            "step=refresh observed=non-zero-start:{:?};needed=clean-wheel",
            initial.points
        )
        .into());
    }

    panel.points[CORE_SLOT] = POINTS;
    let code = panel.build_code();
    panel.preset_book.insert("g02-build", &code)?;
    let path = std::env::temp_dir().join(format!("oteryn-g02-presets-{}.json", std::process::id()));
    panel.preset_book.save(&path)?;
    let loaded = WheelPanel::default().preset_book;
    let reloaded =
        oteryn_dev_client::client_ui::wheel_allocation::presets::PresetBook::load(&path)?;
    let _ = std::fs::remove_file(&path);
    if reloaded == loaded || reloaded.0.get("g02-build") != Some(&code) {
        return Err("step=preset-file observed=preset-not-round-tripped".into());
    }
    panel.points = initial.points;
    panel.import_build(
        reloaded
            .0
            .get("g02-build")
            .ok_or("step=import observed=no-preset")?,
    )?;
    if panel.points[CORE_SLOT] != POINTS {
        return Err("step=import observed=points-not-loaded".into());
    }
    // `apply_result` overwrites `panel.points` with the server answer, so keep the import.
    let imported = panel.points;
    press(&mut client, &mut panel, Intent::Save, 2, "save").await?;
    let saved = panel
        .confirmed_wheel
        .clone()
        .ok_or("step=save observed=no-wheel")?;
    if saved.refused || saved.revision != initial.revision + 1 || saved.points != imported {
        return Err(format!("step=save observed={saved:?};needed=imported-build-confirmed").into());
    }
    logout(&mut client).await?;
    drop(client);

    let mut fresh = connect_character(clients, 0xe2, 0).await?;
    premium_input(&fresh).await?;
    let mut again = admitted_panel(&fresh);
    press(&mut fresh, &mut again, Intent::Refresh, 3, "fresh-refresh").await?;
    let durable = again
        .confirmed_wheel
        .clone()
        .ok_or("step=fresh observed=no-wheel")?;
    if durable.points != imported || durable.revision != saved.revision {
        return Err(format!("step=fresh observed={durable:?};needed={saved:?}").into());
    }
    press(&mut fresh, &mut again, Intent::Reset, 4, "reset").await?;
    let cleared = again
        .confirmed_wheel
        .clone()
        .ok_or("step=reset observed=no-wheel")?;
    if cleared.refused || cleared.points != [0; 36]
        || Some(cleared.revision) != durable.revision.checked_add(1) {
        return Err(format!("step=reset observed={cleared:?};needed=zero-allocation").into());
    }
    logout(&mut fresh).await?;
    drop(fresh);
    let mut reset_read = connect_character(clients, 0xe3, 0).await?;
    premium_input(&reset_read).await?;
    let mut reset_panel = admitted_panel(&reset_read);
    press(&mut reset_read, &mut reset_panel, Intent::Refresh, 5, "reset-relog").await?;
    let persisted = reset_panel.confirmed_wheel.ok_or("step=reset-relog observed=no-wheel")?;
    if persisted.refused || persisted.points != cleared.points || persisted.revision != cleared.revision {
        return Err(format!("step=reset-relog observed={persisted:?};needed={cleared:?}").into());
    }
    logout(&mut reset_read).await
}

async fn premium_input(client: &DevClientSession) -> TestResult {
    character_prep::grant_premium(client).await?;
    evidence(&format!(
        "G02_FIXTURE input=Premium session={}",
        uuid_text(client.join_snapshot().game_session_id.as_bytes())
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_code_round_trips_through_preset_book() -> TestResult {
        let mut panel = WheelPanel::default();
        panel.points[CORE_SLOT] = POINTS;
        let code = panel.build_code();
        panel.preset_book.insert("named", &code)?;
        assert_eq!(panel.preset_book.0.get("named"), Some(&code));
        assert!(panel.preset_book.insert("bad", "not-a-code").is_err());
        Ok(())
    }
}
