//! W10 client capture: native Hazard state before and after a Primal Menace kill, selection,
//! locked-level refusal, and state restored by a fresh admission.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::{attack, hazard, logout::LogoutResult};

const PRIMAL: &str = "oteryn:creature.the_primal_menace";

async fn probe(
    client: &mut DevClientSession,
    root: Option<&Path>,
    key: &str,
    request: &[u8],
    expected_status: CommandStatus,
    expected_payload: &[u8],
) -> TestResult<Vec<u8>> {
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let reply = client
        .session_mut()
        .gated_command(
            hazard::CAPABILITY_HAZARD_V1,
            hazard::COMMAND_TYPE_HAZARD,
            request,
        )
        .await?;
    if reply.status != expected_status || reply.payload != expected_payload {
        return Err(format!(
            "Hazard {key}: {reply:?}; expected {expected_status:?} {expected_payload:?}"
        )
        .into());
    }
    if let Some(root) = root {
        let prefix = format!("w10-{key}");
        let status = reply.status as u32;
        std::fs::write(root.join(format!("{prefix}-request.bin")), request)?;
        std::fs::write(
            root.join(format!("{prefix}-status.bin")),
            status.to_be_bytes(),
        )?;
        std::fs::write(root.join(format!("{prefix}.bin")), &reply.payload)?;
        std::fs::write(
            root.join(format!("{prefix}.status.json")),
            serde_json::to_vec(&serde_json::json!({
                "session_id": session,
                "command_id": reply.command_id,
                "server_sequence": reply.server_sequence,
                "status": status,
            }))?,
        )?;
    }
    Ok(reply.payload)
}

async fn defeat_primal(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("no reachable combat tile for the Primal Menace")?;
    walk(client, world, &path).await?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session.clone(), "oteryn:qualification.w10-primal".into()).await?;
    let target = parse_spawned_entity(&answer)?;
    let visible = tokio::time::Instant::now() + Duration::from_secs(10);
    while !client
        .world_entities()
        .is_some_and(|entities| entities.get(&target).is_some())
    {
        if tokio::time::Instant::now() >= visible {
            return Err("spawned Primal Menace never became visible".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    let identity = uuid_text(&target.identity).replace('-', "");
    let inspection = format!("{identity} {}", target.generation);
    let damage =
        request_fixture_control(session.clone(), "--inspect-damage", inspection.clone()).await?;
    let corpse = damage
        .split_ascii_whitespace()
        .nth(2)
        .ok_or("missing Primal corpse identity")?;
    let corpse = crate::node::operator_files::hex_bytes::<16>(corpse)
        .map_err(|_| "invalid Primal corpse identity")?;
    let modes = attack::FightModes {
        chase: attack::ChaseMode::Chase,
        ..attack::FightModes::DEFAULT
    };
    let modes_reply = client.session_mut().set_fight_modes(&modes).await?;
    if modes_reply.disposition != attack::AttackIntentDisposition::Ok {
        return Err(format!("Primal Menace chase mode refused: {modes_reply:?}").into());
    }
    let mut last_position = client
        .world_entities()
        .and_then(|entities| entities.get(&target))
        .ok_or("Primal target lost before attack")?
        .position;
    let mut last_health = client
        .world_entities()
        .and_then(|entities| entities.get(&target))
        .and_then(|entity| match entity.detail {
            EntityDetail::Actor { health_percent, .. }
            | EntityDetail::ActorWithIcons { health_percent, .. } => Some(health_percent),
            _ => None,
        });
    evidence(&format!(
        "W10_COMBAT initial_target_hp_percent={last_health:?}"
    ));
    let axe = provisioning::item_reference(&provisioning::item_index()?, 7434)?;
    let equipped = client.inventory().is_some_and(|inventory| {
        inventory
            .equipment
            .iter()
            .any(|slot| slot.item.item_definition_ref == axe)
    });
    evidence(&format!(
        "W10_FIXTURE level=1000 axe_skill=120 weapon=royal_axe_7434 equipped={equipped} damage_type=physical primal_starting_hp=1000"
    ));
    let reply = client.session_mut().attack_target(Some(&target)).await?;
    if !equipped {
        return Err("W10 fixture royal axe is not equipped".into());
    }
    if reply.status != CommandStatus::Accepted
        || reply.disposition != attack::AttackIntentDisposition::Ok
    {
        return Err(format!("Primal Menace attack refused: {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    verdict::Harness::global().bounded_wait(deadline, "W10-Primal-combat");
    let mut removed = false;
    let mut inspected = false;
    let state_deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut target_confirmed = false;
    loop {
        for event in client.take_events() {
            if let SessionEvent::ActorCombatState(delta) = &event
                && delta.server_sequence > reply.result_server_sequence
                && !target_confirmed
            {
                if delta.value.target != Some(target) {
                    return Err(format!(
                        "first post-attack combat delta has wrong target: {:?}",
                        delta.value
                    )
                    .into());
                }
                target_confirmed = true;
                evidence(&format!(
                    "W10_COMBAT attack_status=Accepted target_set=true sequence={}",
                    delta.server_sequence
                ));
            }
            if let SessionEvent::WorldSpatialEntities(delta) = event {
                if delta.value.leave.contains(&target) {
                    removed = true;
                    evidence(&format!(
                        "W10_COMBAT target_left_view=true target={last_position:?} player={:?} sequence={}",
                        delta.value.actor_position, delta.server_sequence
                    ));
                }
                for entity in delta.value.enter.iter().chain(&delta.value.update) {
                    if entity.entity == target && entity.position != last_position {
                        evidence(&format!(
                            "W10_COMBAT target_moved from={last_position:?} to={:?} player={:?}",
                            entity.position, delta.value.actor_position
                        ));
                        last_position = entity.position;
                    }
                    if entity.entity == target
                        && let EntityDetail::Actor { health_percent, .. }
                        | EntityDetail::ActorWithIcons { health_percent, .. } = entity.detail
                        && last_health != Some(health_percent)
                    {
                        evidence(&format!(
                            "W10_COMBAT target_hp_percent={health_percent} previous={last_health:?}"
                        ));
                        last_health = Some(health_percent);
                    }
                }
            }
        }
        if !inspected && tokio::time::Instant::now() >= deadline - Duration::from_secs(85) {
            inspected = true;
            let damage =
                request_fixture_control(session.clone(), "--inspect-damage", inspection.clone())
                    .await;
            evidence(&format!("W10_COMBAT live_damage={damage:?}"));
        }
        if !target_confirmed && tokio::time::Instant::now() >= state_deadline {
            return Err("W10 attack accepted without its combat-state delta".into());
        }
        let corpse_seen = client.world_entities().is_some_and(|entities| {
            entities
                .iter()
                .any(|entity| entity.kind == EntityKind::Corpse && entity.entity.identity == corpse)
        });
        if corpse_seen && target_confirmed {
            evidence(&format!(
                "W10_COMBAT exact_corpse=true target_removed={removed} last_hp_percent={last_health:?}"
            ));
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("Primal death unconfirmed: target_removed={removed} exact_corpse=false last_hp_percent={last_health:?} actor_vitals={:?}", client.actor_vitals()).into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    verdict::Harness::global().bounded_wait(deadline, "W10-Primal-death-receipt");
    committed_hunt_death(
        client,
        clients,
        Some(clients.characters[W10_CHARACTER]),
        PRIMAL,
        target,
        deadline,
    )
    .await
    .map_err(|error| format!("BLOCKED-SERVER: exact Primal corpse observed: {error}"))?;
    client.session_mut().attack_target(None).await?;
    use oteryn_protocol_oteryn::quick_loot as loot;
    let request = loot::encode_quick_loot_request(&loot::QuickLootRequest {
        action: loot::QuickLootAction::Click,
        corpse: Some(corpse),
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        filter_mode: None,
        filter_definition_ref: None,
    })
    .map_err(|error| format!("Primal loot request: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            loot::CAPABILITY_QUICK_LOOT_V1,
            loot::COMMAND_TYPE_QUICK_LOOT,
            &request,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("first Primal corpse loot refused: {reply:?}").into());
    }
    let loot = loot::decode_quick_loot_result(&reply.payload)
        .map_err(|error| format!("Primal loot result: {error:?}"))?;
    if !loot.items.is_empty() {
        return Err(format!("Primal corpse had unexpected physical loot: {loot:?}").into());
    }
    evidence("W10_LOOT exact_corpse_quick_loot=accepted physical_items=0");
    Ok(())
}

async fn reset_kept_hazard_fixture(clients: &SeamClients<'_>) -> TestResult {
    wait_character_released(clients, W10_CHARACTER).await?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let mut transaction = db.begin().await?;
    // Only this disposable fixture goes back to level one. The native kill
    // and selection still run with the ordinary progression guards enabled.
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "UPDATE game_character_hazard AS hazard SET unlocked = 1, selected = 1
         FROM game_character_roots AS root
         WHERE hazard.character_id = root.character_id
           AND root.character_id = encode($1, 'hex')::uuid
           AND root.world_id = encode($2, 'hex')::uuid
           AND hazard.unlocked = 2 AND hazard.selected IN (1, 2)",
    )
    .bind(clients.characters[W10_CHARACTER].as_slice())
    .bind(clients.world.as_slice())
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR");
    let root = root.as_deref().map(Path::new);
    let bundle = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bundle,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    reset_kept_hazard_fixture(clients).await?;
    let mut client = connect_character(clients, 0x05, W10_CHARACTER).await?;
    let staged = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(staged.as_bytes()),
        "--profile",
        "w10-primal".to_owned(),
    )
    .await?;
    if logout_retrying_busy(&mut client).await?.0 != Some(LogoutResult::Accepted) {
        return Err("W10 fixture logout refused".into());
    }
    drop(client);
    let mut client = connect_character(clients, 0x06, W10_CHARACTER).await?;
    if client.join_snapshot().game_session_id == staged {
        return Err("W10 fixture readmission reused its staging session".into());
    }
    let (health, mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "knight",
        1000,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("W10 fixture maxima unavailable")?;
    if !client
        .actor_vitals()
        .is_some_and(|vitals| vitals.max_health == health && vitals.max_mana == mana)
    {
        return Err("W10 fixture readmission did not load level-1000 knight vitals".into());
    }
    let weapon = provisioning::item_reference(&provisioning::item_index()?, 7434)?;
    let axe_equipped = |client: &DevClientSession| {
        client.inventory().is_some_and(|inventory| {
            inventory
                .equipment
                .iter()
                .any(|slot| slot.item.item_definition_ref == weapon)
        })
    };
    // The equip hotkey toggles an already-worn axe off.
    if !axe_equipped(&client) {
        let equip = client.session_mut().equip_item_definition(weapon).await?;
        if equip.status != CommandStatus::Accepted
            || equip.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
        {
            return Err(format!("W10 fixture royal axe equip refused: {equip:?}").into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while !axe_equipped(&client) {
            if tokio::time::Instant::now() >= deadline {
                return Err("W10 fixture royal axe equipment delta missing".into());
            }
            client.service_liveness(Duration::from_millis(100)).await?;
            client.take_events();
        }
    }
    let session = client.join_snapshot().game_session_id;
    use CommandStatus::{Accepted, Rejected};
    probe(&mut client, root, "initial", &[], Accepted, &[1, 1]).await?;
    defeat_primal(&mut client, clients, world).await?;
    let query = probe(&mut client, root, "query", &[], Accepted, &[2, 1]).await?;
    let select = probe(&mut client, root, "select", &[2], Accepted, &[2, 2]).await?;
    probe(&mut client, root, "locked", &[3], Rejected, &[]).await?;
    probe(&mut client, root, "unchanged", &[], Accepted, &[2, 2]).await?;

    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| is_logout_rest_tile(world, at))
        .ok_or("no reachable logout rest tile after the Primal Menace fight")?;
    walk(&mut client, world, &path).await?;
    let (result, observed, _, _) = logout_retrying_busy(&mut client).await?;
    if result != Some(LogoutResult::Accepted) {
        return Err(format!("Hazard logout refused: {observed}").into());
    }
    drop(client);
    let mut client = connect_character(clients, 0x06, W10_CHARACTER).await?;
    if client.join_snapshot().game_session_id == session {
        return Err("Hazard readmission reused the original session".into());
    }
    probe(&mut client, root, "relog", &[], Accepted, &[2, 2]).await?;
    let expected = format!(
        "query_unlocked={}\nquery_selected={}\nselect_unlocked={}\nselect_selected={}\n",
        query[0], query[1], select[0], select[1],
    );
    if let Some(root) = root {
        std::fs::write(root.join("w10-expected.txt"), &expected)?;
    }
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W10.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W10.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W10.client status=FAIL evidence=timeout"),
    }
}
