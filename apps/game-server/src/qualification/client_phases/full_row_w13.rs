//! W13 player scenario, derived from the matrix/re-audit and the daily-boost,
//! kill-reward, spawn-realization, and world-respawn owners:
//! 1. Admit the provisioned character and send DAILY_BOOST_QUERY; expect an
//!    accepted view naming the day's creature, boss, and reset epoch.
//! 2. Send the query again; expect the identical selection (the stable-day path).
//! 3. Send a malformed DAILY_BOOST_QUERY payload; expect a rejected reply and no view.
//! 4. Walk to the selected creature's authored spawn, attack it until death, open
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

async fn approach_boosted_creature(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    creature: &str,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let points: Vec<_> = spawns
        .sources
        .iter()
        .flat_map(|source| source.points.iter().map(move |point| (source.floor, point)))
        .filter(|(_, point)| {
            spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|row| row.key == creature)
        })
        .collect();
    let path = served_route(world, from, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
            && points.iter().any(|(floor, point)| {
                at.floor == *floor && at.x.abs_diff(point.x).max(at.y.abs_diff(point.y)) <= 1
            })
    })
    .ok_or_else(|| {
        format!("no connected served route from {from:?} to an authored {creature} spawn")
    })?;
    walk(client, world, &path).await?;
    // Preserve the normal follow-to-melee path once within the spawn neighborhood.
    travel_to_spawn(client, world, spawns, creature).await
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
    client_hunt::hunt(client).await.map_err(|error| {
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

async fn prepare_hunter(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
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

    let mut client = connect_character(clients, 0xd3, 0).await?;
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
    let mut client = prepare_hunter(clients).await?;
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

    if let Err(error) =
        approach_boosted_creature(&mut client, world, &metadata.spawns, &selected.creature_key)
            .await
    {
        if matches!(selected.boss_key.as_str(), BAELOC | NICTROS) {
            match enter_grave(&mut client, world).await {
                Ok(()) => evidence(
                    "W13_CHECK Grave Danger entry verified independently of creature route",
                ),
                Err(entry) => evidence(&format!("W13_GRAVE_ENTRY {entry}")),
            }
        }
        return Err(verdict::context(
            format!("step-4: selected creature route/spawn observed {error}"),
            error,
        ));
    }
    required_hunt(&mut client, 4, "selected creature").await?;
    evidence("W13_CHECK step=4 selected-creature-killed-and-looted");

    // A second real approach waits through the owner's respawn mechanism rather
    // than manufacturing an actor. The bounded helper reports the missing actor.
    approach_boosted_creature(&mut client, world, &metadata.spawns, &selected.creature_key)
        .await
        .map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            verdict::context(
                format!("step-5: boosted creature respawn observed {error}"),
                error,
            )
        })?;
    let step_5_observed = required_hunt(&mut client, 5, "respawned selected creature").await?;

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
    let step_6_observed = required_hunt(&mut client, 6, "selected boss").await?;

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
