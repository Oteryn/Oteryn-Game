//! W13 player scenario, derived from the matrix/re-audit and the daily-boost,
//! kill-reward, spawn-realization, and world-respawn owners:
//! 1. Admit the provisioned character and send DAILY_BOOST_QUERY; expect an
//!    accepted view naming the day's creature, boss, and reset epoch.
//! 2. Send the query again; expect the identical selection (the stable-day path).
//! 3. Send a malformed DAILY_BOOST_QUERY payload; expect a rejected reply and no view.
//! 4. Admit a dedicated creature through the spawn fixture, attack it until death, open
//!    its corpse and move real loot; expect combat/entity/container/inventory deltas.
//! 5. Observe the selected creature respawn, kill and loot it again; the owner must
//!    apply boosted creature XP and extra loot rolls without a player shortcut.
//! 6. Walk to the selected boss, fight and loot it; expect boss bonus loot while
//!    its unique drop is not duplicated, followed by its owner-driven respawn clock.
//! 7. Log out, obtain a fresh admission, query again, and expect the same durable
//!    reset epoch and creature/boss selection.
//!
//! A runtime failure records the first uncompleted step and the actual reply or
//! missing delta. The journey never substitutes a source-only verdict for an attempt.
use super::*;
use oteryn_dev_client::client_runtime::raw_feature;
use oteryn_dev_client::client_ui::daily_boost::{self, DailyBoostPanel, Intent};
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::{boss_encounter as boss, daily_boost as wire};

const BAELOC: &str = "oteryn:creature.sir_baeloc";
const NICTROS: &str = "oteryn:creature.sir_nictros";
const GRAVE_LEVER: &str = "oteryn:boss.sir_nictros";

// Crystal actions_sir_baeloc_sir_nictros_lever.lua. The authored entry
// selects the two-actor encounter through the Sir Nictros lever key.
fn grave_slot(at: TilePos) -> bool {
    at.floor == -13 && at.y == 31_413 && (33_424..=33_428).contains(&at.x)
}

fn served_route(
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
) -> Option<Vec<StepDirection>> {
    find_route(from, goal, |at, direction| {
        let next = destination(world, at, direction)?;
        // W13 searches only connected, served cells, with a row-local envelope
        // that includes the Grave Danger room and the served Braindeath spawns.
        (next.x.abs_diff(from.x) <= 1_200
            && next.y.abs_diff(from.y) <= 1_200
            && next.floor.abs_diff(from.floor) <= 8)
            .then_some(next)
    })
    .ok()
}

async fn enter_grave(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = served_route(world, from, grave_slot).ok_or_else(|| {
        format!("step-6: no connected served route from {from:?} to the Grave Danger lever slots")
    })?;
    walk(client, world, &path).await?;
    verdict::Harness::global().progress("grave-lever");
    let origin = client.join_snapshot().game_session_id;
    let payload = boss::encode_boss_key(GRAVE_LEVER).map_err(|error| format!("{error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            boss::CAPABILITY_BOSS_ENCOUNTER_V1,
            boss::COMMAND_TYPE_BOSS_LEVER,
            &payload,
        )
        .await?;
    let decoded = boss::decode_boss_activity_reply(&reply.payload);
    let Ok(boss::BossActivityReply::Accepted { handoff, .. }) = decoded else {
        return Err(format!(
            "step-6: Grave Danger lever returned {:?}: {decoded:?}",
            reply.status
        )
        .into());
    };
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step-6: Grave Danger handoff status {:?}", reply.status).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let current = client.join_snapshot().game_session_id;
        if current != origin {
            let scope = client
                .session_mut()
                .activity_scope()
                .ok_or("step-6: missing Grave Danger scope")?;
            if scope.handoff != handoff
                || scope.source_session != *origin.as_bytes()
                || scope.session != *current.as_bytes()
                || !matches!(
                    scope.scope,
                    oteryn_protocol_oteryn::activity_scope::ActivityScope::Instance { .. }
                )
            {
                return Err(format!("step-6: wrong Grave Danger handoff {scope:?}").into());
            }
            if client.world_entities().is_some_and(|view| {
                ["Sir Baeloc", "Sir Nictros"].iter().all(|name| {
                    view.iter().any(|actor| {
                        actor.kind == EntityKind::Creature
                            && actor
                                .display_name
                                .is_some_and(|display| display.as_str() == Ok(*name))
                    })
                })
            }) {
                evidence("W13_CHECK step=6 native-Grave-Danger-handoff-and-boss-pair");
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step-6: Grave Danger handoff did not expose both authored bosses".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

const BRAINDEATH: &str = "oteryn:creature.braindeath";
const BRAINDEATH_SOURCE: &str = "oteryn:spawn.x32077_y31832_z9";
const BRAINDEATH_HOME: TilePos = TilePos {
    x: 32075,
    y: 31829,
    floor: -9,
};

fn fixture_approach(
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<TilePos> {
    let home = BRAINDEATH_HOME;
    let source = spawns
        .sources
        .iter()
        .find(|source| source.key == BRAINDEATH_SOURCE)
        .ok_or("W13 authored spawn source absent from served bundle")?;
    if source.floor != home.floor
        || !source.points.iter().any(|point| {
            point.x == home.x
                && point.y == home.y
                && spawns
                    .creatures
                    .get(point.creature as usize)
                    .is_some_and(|row| row.key == BRAINDEATH)
        })
    {
        return Err("W13 authored Braindeath spawn point absent from served bundle".into());
    }
    // Select one fixture cell from the map before admission or any combat.
    // Cardinal neighbors avoid an attack across a diagonal wall.
    [(0, -1), (1, 0), (0, 1), (-1, 0)]
        .into_iter()
        .map(|(dx, dy)| TilePos {
            x: home.x.saturating_add_signed(dx),
            y: home.y.saturating_add_signed(dy),
            floor: home.floor,
        })
        .find(|at| {
            world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|ground| ground.walkable() && !ground.protection_zone())
                && world.enterable(*at)
                && !spawns.sources.iter().any(|source| {
                    source.floor == at.floor
                        && source
                            .points
                            .iter()
                            .any(|point| point.x == at.x && point.y == at.y)
                })
        })
        .ok_or_else(|| {
            "W13 authored Braindeath has no walkable unoccupied cardinal approach".into()
        })
}

async fn stage_hunter(clients: &SeamClients<'_>, approach: TilePos) -> TestResult {
    wait_character_released(clients, 0).await?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let changed = sqlx::query(
        "UPDATE game_character_last_positions AS p
         SET tile_x=$3, tile_y=$4, tile_floor=$5, write_sequence=write_sequence+1
         WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid
           AND (p.tile_x,p.tile_y,p.tile_floor) IS DISTINCT FROM ($3,$4,$5)
           AND NOT EXISTS (
               SELECT 1 FROM game_durability_reconnect_sessions AS s
               WHERE s.character_id=p.character_id AND s.world_id=p.world_id AND s.session_state<>3
           )",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .bind(i32::from(approach.x))
    .bind(i32::from(approach.y))
    .bind(i32::from(-approach.floor))
    .execute(&mut db)
    .await?;
    db.close().await?;
    if changed.rows_affected() != 1 {
        return Err("W13 fixture requires one different saved offline world position".into());
    }
    Ok(())
}

pub(super) async fn query(
    client: &mut oteryn_dev_client::DevClientSession,
    step: u8,
) -> TestResult<wire::DailyBoostView> {
    let mut panel = DailyBoostPanel {
        selected: true,
        ..Default::default()
    };
    let command = daily_boost::request(
        &mut panel,
        Intent {
            command: wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
            payload: Vec::new(),
        },
    )
    .ok_or_else(|| format!("step-{step}: client did not encode DAILY_BOOST_QUERY"))?;
    let reply = raw_feature::send(client.session_mut(), &command).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step-{step}: DAILY_BOOST_QUERY observed reply {reply:?}").into());
    }
    daily_boost::apply_result(&mut panel, &reply);
    panel
        .boost
        .ok_or_else(|| format!("step-{step}: accepted query had no decoded view delta").into())
}

async fn required_hunt(
    client: &mut oteryn_dev_client::DevClientSession,
    step: u8,
    target: &str,
    exact: Option<oteryn_dev_client::WorldSpatialEntity>,
) -> TestResult<String> {
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)
    {
        return Err(format!(
            "step-{step}: {target} hunt cannot run; admission omitted world-map capability"
        )
        .into());
    }
    let before = client
        .inventory()
        .cloned()
        .ok_or_else(|| format!("step-{step}: {target} hunt had no inventory before combat"))?;
    let result = match exact {
        Some(entity) => {
            let position = tile(entity.position)?;
            let active = client.world_entities()
                .and_then(|view| view.get(&entity.entity))
                .copied()
                .ok_or_else(|| format!("W13 hunt expected visible identity={:?} position={position:?} health=100%; observed absent", entity.entity))?;
            confirm_braindeath(&active, entity.entity, position)?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(85);
            match tokio::time::timeout_at(
                deadline,
                client_hunt::hunt_target(client, entity, Some(Duration::from_secs(75))),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => {
                    let observed = client
                        .world_entities()
                        .and_then(|view| view.get(&entity.entity));
                    Err(format!("hunt deadline waiting for damage, death and loot: target={:?} observed={observed:?} player={:?} open_container={:?}",
                        entity.entity, client.world_spatial().actor_position, client.open_container()).into())
                }
            }
        }
        None => client_hunt::hunt(client).await,
    };
    result.map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        verdict::context(
            format!("step-{step}: {target} combat/loot observed {error}"),
            error,
        )
    })?;
    let after = client
        .inventory()
        .cloned()
        .ok_or_else(|| format!("step-{step}: {target} hunt had no inventory after looting"))?;
    if after == before {
        return Err(format!(
            "step-{step}: {target} hunt returned Ok but observed no inventory delta; inventory={after:?}"
        )
        .into());
    }
    Ok(format!(
        "hunt=Ok inventory-before={before:?} inventory-after={after:?}"
    ))
}

fn confirm_braindeath(
    actor: &oteryn_dev_client::WorldSpatialEntity,
    identity: oteryn_dev_client::EntityRef,
    expected: TilePos,
) -> TestResult {
    let observed = tile(actor.position)?;
    let name = actor
        .display_name
        .as_ref()
        .and_then(|name| name.as_str().ok());
    let health = match actor.detail {
        oteryn_dev_client::EntityDetail::Actor { health_percent, .. } => Some(health_percent),
        _ => None,
    };
    if actor.entity != identity
        || actor.kind != EntityKind::Creature
        || name != Some("Braindeath")
        || observed != expected
        || health != Some(100)
    {
        return Err(format!(
            "W13 Braindeath target expected identity={identity:?} kind=Creature name=Braindeath position={expected:?} health=100%; observed identity={:?} kind={:?} name={name:?} position={observed:?} health={health:?}",
            actor.entity, actor.kind
        ).into());
    }
    Ok(())
}

async fn fixture_braindeath(
    client: &mut DevClientSession,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    client.take_events();
    let player = tile(client.world_spatial().actor_position)?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session, BRAINDEATH.into()).await?;
    let target = parse_spawned_entity(&answer)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            let oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta) = event else {
                continue;
            };
            for actor in delta.value.enter {
                if actor.entity != target {
                    continue;
                }
                let position = tile(actor.position)?;
                confirm_braindeath(&actor, target, position)?;
                if position.floor != player.floor
                    || position
                        .x
                        .abs_diff(player.x)
                        .max(position.y.abs_diff(player.y))
                        != 1
                    || spawns.sources.iter().any(|source| {
                        source.floor == position.floor
                            && source
                                .points
                                .iter()
                                .any(|point| point.x == position.x && point.y == position.y)
                    })
                {
                    return Err(format!("W13 fixture expected clear adjacent position to {player:?}; observed {position:?} target={target:?}").into());
                }
                evidence(&format!(
                    "FIXTURE row=W13.full kind=spawn creature={BRAINDEATH} target={target:?} position={position:?}"
                ));
                return Ok(actor);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            let observed = client.world_entities().and_then(|view| view.get(&target));
            return Err(format!("W13 deadline waiting for dedicated Braindeath enter frame: target={target:?} observed={observed:?} player={player:?}").into());
        }
    }
}

async fn prepare_hunter(
    clients: &SeamClients<'_>,
    approach: TilePos,
) -> TestResult<DevClientSession> {
    use oteryn_protocol_oteryn::{cyclopedia_character as stats, logout::LogoutResult};

    character_prep::declare_seeded("W13.full", "knight-level-200", "daily boost combat");
    let mut initial = connect_character(clients, 0xd3, 0).await?;
    let session = initial.join_snapshot().game_session_id;
    request_fixture_control(uuid_text(session.as_bytes()), "--profile", "q_w13".into()).await?;
    let logout = initial.session_mut().logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("W13 preparation logout refused: {:?}", logout.result).into());
    }
    drop(initial);
    stage_hunter(clients, approach).await?;

    let mut client = connect_character(clients, 0xd3, 0).await?;
    let admitted = tile(client.world_spatial().actor_position)?;
    if admitted != approach {
        return Err(format!("W13 fixture position {admitted:?}, expected {approach:?}").into());
    }
    evidence(&format!(
        "W13_CHECK fixture source={BRAINDEATH_SOURCE} home={BRAINDEATH_HOME:?} approach={approach:?}"
    ));
    if client.join_snapshot().game_session_id == session {
        return Err("W13 preparation reused its original session".into());
    }
    let reply = client
        .session_mut()
        .gated_command(
            stats::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            stats::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[stats::CharacterTab::GeneralStats as u8, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("W13 seeded GeneralStats refused: {:?}", reply.status).into());
    }
    let data = &reply.payload;
    let vocation = general_stats::validate(data, general_stats::selected(&client))?;
    let level = u32::from_le_bytes(data[9..13].try_into()?);
    let xp = u64::from_le_bytes(data[1..9].try_into()?);
    if vocation != stats::vocation_id("knight") || level != 200 || xp != 129_389_800 {
        return Err(format!(
            "W13 seeded stats mismatch: vocation={vocation:?} level={level} xp={xp}"
        )
        .into());
    }
    verdict::Harness::global().progress("seeded-hunter");
    evidence(&format!("W13_CHECK seeded-knight level={level} xp={xp}"));
    Ok(client)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_Q_W13_ACCOUNT").await?;
    let clients = &dedicated.clients(clients);
    let approach = fixture_approach(world, &metadata.spawns)?;
    let mut client = prepare_hunter(clients, approach).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_DAILY_BOOST_V1)
    {
        return Err("step-1: admission did not offer daily-boost capability 151".into());
    }
    let selected = query(&mut client, 1).await?;
    evidence(&format!(
        "W13_CHECK step=1 accepted epoch={} creature={} boss={}",
        selected.reset_epoch, selected.creature_key, selected.boss_key
    ));
    let repeated = query(&mut client, 2).await?;
    if repeated != selected {
        return Err(format!(
            "step-2: repeated query changed view from {selected:?} to {repeated:?}"
        )
        .into());
    }
    let refused = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_BOOST_V1,
            wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
            &[0],
        )
        .await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!("step-3: malformed query observed reply {refused:?}").into());
    }
    evidence("W13_CHECK step=3 malformed-query-rejected");

    if selected.creature_key != BRAINDEATH {
        return Err(format!(
            "W13 fixture targets Braindeath, selected {}",
            selected.creature_key
        )
        .into());
    }
    verdict::Harness::global().progress("daily-boost-verified");
    let first = fixture_braindeath(&mut client, &metadata.spawns).await?;
    let home = tile(first.position)?;
    verdict::Harness::global().progress("dedicated-braindeath-visible");
    required_hunt(&mut client, 4, "selected creature", Some(first)).await?;
    verdict::Harness::global().progress("braindeath-killed-and-looted");
    evidence("W13_CHECK step=4 selected-creature-killed-and-looted");

    // The fixture owns this cell. Require a new live enter there after the first kill.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(100);
    verdict::Harness::global().bounded_wait(deadline, "dedicated-braindeath-respawn");
    let respawned = 'respawn: loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            let oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta) = event else {
                continue;
            };
            for actor in delta.value.enter {
                if actor.kind == EntityKind::Creature && tile(actor.position)? == home {
                    if actor.entity == first.entity {
                        return Err(format!(
                            "W13 respawn expected a new identity; observed {:?}",
                            actor.entity
                        )
                        .into());
                    }
                    confirm_braindeath(&actor, actor.entity, home)?;
                    break 'respawn actor;
                }
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("W13 deadline waiting for dedicated Braindeath respawn at {home:?}; previous identity={:?}", first.entity).into());
        }
    };
    verdict::Harness::global().progress("braindeath-respawned");
    let step_5_observed = required_hunt(
        &mut client,
        5,
        "respawned selected creature",
        Some(respawned),
    )
    .await?;
    verdict::Harness::global().progress("respawned-braindeath-killed-and-looted");

    if matches!(selected.boss_key.as_str(), BAELOC | NICTROS) {
        enter_grave(&mut client, world).await?;
        return Err("step-6: native Grave Danger entry verified, selected boss loot and respawn evidence still required".into());
    }
    travel_to_spawn(&mut client, world, &metadata.spawns, &selected.boss_key)
        .await
        .map_err(|error| {
            verdict::context(
                format!("step-6: selected boss route/spawn observed {error}"),
                error,
            )
        })?;
    let step_6_observed = required_hunt(&mut client, 6, "selected boss", None).await?;

    // The player protocol currently exposes neither the XP award nor which
    // loot rolls were bonus/unique, and it publishes no boss-respawn clock.
    // A successful generic hunt therefore cannot prove either numbered step.
    let unobservable_combat_deltas = format!(
        "step-5: boosted XP and extra-loot deltas are not observable; observed {step_5_observed}; \
         step-6: boss bonus/unique-loot and respawn-clock deltas are not observable; observed {step_6_observed}"
    );

    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-7: logout reply was not Accepted".into());
    }
    let mut fresh = connect_character(clients, 0xd4, 0).await?;
    let durable = query(&mut fresh, 7).await?;
    if durable != selected {
        return Err(format!(
            "step-7: fresh admission changed durable view from {selected:?} to {durable:?}"
        )
        .into());
    }
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-7: fresh-admission logout reply was not Accepted".into());
    }
    evidence("W13_CHECK step=7 durable-selection-rechecked");
    Err(unobservable_combat_deltas.into())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients)))
            .await;
    let (status, gap) = match result {
        Ok(Ok(())) => ("PASS", None),
        Ok(Err(error)) => ("FAIL", Some(error.to_string())),
        Err(_) => (
            "FAIL",
            Some("step-timeout: no required reply/delta within 420s".into()),
        ),
    };
    if let Some(gap) = gap {
        evidence(&format!("W13_GAP {gap}"));
    }
    evidence(&format!(
        "e2e_all scenario=W13.full status={status} seconds={:.3}",
        started.elapsed().as_secs_f64()
    ));
}
