//! P23c: modern Task Board reward progress plus retired legacy negative controls.

use super::*;
use oteryn_dev_client::{DevClientSession, client_runtime::task_board};
use oteryn_protocol_oteryn::{logout::LogoutResult, task_board as wire};

const ROW: &str = "P23c";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(600), verdict::observe(journey(clients)))
            .await;
    let failure = match result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => Some("step=hunting-task-journey observed=600s-timeout".to_owned()),
    };
    match failure {
        None => evidence(&format!(
            "e2e_all scenario={ROW}.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Some(error) => {
            evidence(&format!("P23C_GAP {error}"));
            evidence(&format!(
                "e2e_all scenario={ROW}.full status=FAIL seconds={:.3} evidence={}",
                started.elapsed().as_secs_f64(),
                token(&error)
            ));
        }
    }
}

async fn command(
    client: &mut DevClientSession,
    action: u8,
    task: u8,
) -> TestResult<task_board::TaskBoardResponse> {
    task_board::send(
        client.session_mut(),
        &task_board::TaskBoardCommand(wire::TaskBoardRequest { action, task }),
    )
    .await
    .map_err(Into::into)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    use super::client_row_p23c::{connect_seeded, exchange};
    let character = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_P23C_ACCOUNT",
        Some("Weekly Task Hunter"),
    )
    .await?;
    let clients = &character.clients(clients);
    let mut client = connect_seeded(clients, 0xc3, "P23c.full").await?;
    let (initial, _) = exchange(&mut client, 4, 0).await?;
    if initial.refusal != 0 || initial.weekly.started || initial.bounty.active.is_some() {
        return Err(format!("step=modern-query observed={initial:?}").into());
    }
    check_legacy_controls(&mut client, &initial).await?;

    let (weekly, _) = exchange(&mut client, 5, 0).await?;
    let task = weekly
        .weekly
        .kills
        .iter()
        .find(|task| !task.race.is_empty())
        .ok_or_else(|| {
            format!(
                "step=weekly-start refusal={} kill-tasks={:?}",
                weekly.refusal, weekly.weekly.kills
            )
        })?;
    if weekly.refusal != 0
        || !weekly.weekly.started
        || weekly.weekly.difficulty != 0
        || weekly.weekly.pending_points != 0
        || weekly.points != initial.points
        || weekly.bounty.state != 0
        || !weekly.bounty.offers.is_empty()
        || task.current != 0
        || task.required == 0
    {
        return Err(format!("step=weekly-start observed={weekly:?}").into());
    }

    let world = full_e2ef7::qualification_world(clients)?;
    prepare_hunt_ground(&mut client, world).await?;
    for current in 1..=task.required {
        credit_one_kill(&mut client, clients, &task.race).await?;
        let (progress, _) = exchange(&mut client, 4, 0).await?;
        let expected_pending = if current == task.required { 25 } else { 0 };
        if progress.refusal != 0
            || progress.points != initial.points
            || progress.weekly.pending_points != expected_pending
            || !progress.weekly.kills.iter().any(|observed| {
                observed.race == task.race
                    && observed.required == task.required
                    && observed.current == current
            })
        {
            return Err(format!("step=weekly-credit expected={current} pending={expected_pending} observed={progress:?}").into());
        }
    }

    let (opened, _) = exchange(&mut client, 7, 0).await?;
    let offer = opened
        .bounty
        .offers
        .first()
        .ok_or_else(|| format!("step=bounty-open refusal={} offers=0", opened.refusal))?;
    let (selection, _) = exchange(&mut client, 8, 0).await?;
    if opened.refusal != 0
        || opened.bounty.state != 1
        || opened.bounty.active.is_some()
        || selection.refusal != 0
        || selection.bounty.state != 2
        || selection.bounty.selected != 1
        || selection.bounty.current != 0
        || selection.bounty.active.as_ref() != Some(offer)
        || offer.required == 0
        || offer.points == 0
    {
        return Err(format!("step=bounty-select observed={selection:?}").into());
    }
    check_legacy_controls(&mut client, &selection).await?;
    let (early, _) = exchange(&mut client, 9, 0).await?;
    if early
        != (wire::full::Reply {
            refusal: 2,
            ..selection.clone()
        })
    {
        return Err(format!("step=bounty-early-claim observed={early:?}").into());
    }
    let mut completed = selection.clone();
    for current in 1..=offer.required {
        credit_one_kill(&mut client, clients, &offer.race).await?;
        let (progress, _) = exchange(&mut client, 4, 0).await?;
        let state = if current == offer.required { 3 } else { 2 };
        if progress.refusal != 0
            || progress.bounty.state != state
            || progress.bounty.current != current
            || progress.bounty.active.as_ref() != Some(offer)
            || progress.bounty_points != initial.bounty_points
        {
            return Err(
                format!("step=bounty-credit expected={current} observed={progress:?}").into(),
            );
        }
        completed = progress;
    }
    let (claimed, _) = exchange(&mut client, 9, 0).await?;
    let expected_points = initial
        .bounty_points
        .checked_add(u64::from(offer.points))
        .ok_or("bounty balance overflow")?;
    if claimed.refusal != 0
        || claimed.bounty.state != 1
        || claimed.bounty.active.is_some()
        || claimed.bounty.current != 0
        || claimed.bounty_points != expected_points
        || claimed.points != initial.points
        || claimed.weekly != completed.weekly
        || claimed.soulseals != completed.soulseals
        || claimed.bounty.rerolls
            != completed
                .bounty
                .rerolls
                .saturating_add(1)
                .min(oteryn_gameplay_quests::quest::task_board::full::MAX_REROLL_TOKENS)
    {
        return Err(format!(
            "step=bounty-claim expected-bounty-points={expected_points} observed={claimed:?}"
        )
        .into());
    }
    let (duplicate, _) = exchange(&mut client, 9, 0).await?;
    if duplicate
        != (wire::full::Reply {
            refusal: 2,
            ..claimed.clone()
        })
    {
        return Err(format!("step=bounty-duplicate-claim observed={duplicate:?}").into());
    }
    let session = client.join_snapshot().game_session_id;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    verdict::Harness::global().bounded_wait(deadline, "task-board-combat-lock");
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(250)).await?;
        client.take_events();
    }
    logout(&mut client).await?;
    let mut relog = connect_character(clients, 0xc4, 0).await?;
    let (persisted, _) = exchange(&mut relog, 4, 0).await?;
    logout(&mut relog).await?;
    if relog.join_snapshot().game_session_id == session {
        return Err("step=task-board-relog reused the previous game session".into());
    }
    if persisted.refusal != 0
        || persisted.bounty != claimed.bounty
        || persisted.bounty_points != claimed.bounty_points
    {
        return Err(
            format!("step=bounty-relog observed={persisted:?} expected={claimed:?}").into(),
        );
    }
    evidence(&format!(
        "P23C_CHECK step=bounty-relog bounty-points={}",
        persisted.bounty_points
    ));
    verify_settlement(&claimed, &persisted)
}

async fn check_legacy_controls(
    client: &mut DevClientSession,
    modern: &wire::full::Reply,
) -> TestResult {
    let bounty = modern
        .bounty
        .active
        .as_ref()
        .map_or(modern.bounty.offers.as_slice(), std::slice::from_ref);
    let tasks: Vec<_> = bounty
        .iter()
        .map(|task| (&task.race, task.required))
        .chain(
            modern
                .weekly
                .kills
                .iter()
                .filter(|task| !task.race.is_empty())
                .map(|task| (&task.race, task.required)),
        )
        .take(wire::MAX_TASKS)
        .map(|(race, required)| wire::TaskBoardTask {
            race: race.clone(),
            required: i64::from(required),
            claimed_at_unix: 0,
        })
        .collect();
    let query = command(client, 0, 0).await?;
    if query.status != CommandStatus::Accepted
        || query.reply.refused
        || query.reply.tasks != tasks
        || (query.reply.selected, query.reply.kills) != (0, 0)
        || u64::try_from(query.reply.points)? != modern.points
        || u64::try_from(query.reply.bounty_points)? != modern.bounty_points
    {
        return Err(
            format!("step=compact-projection expected-tasks={tasks:?} observed={query:?}").into(),
        );
    }

    for action in [1, 2] {
        let refused = command(client, action, 1).await?;
        if refused.status != CommandStatus::Accepted
            || refused.reply
                != (wire::TaskBoardReply {
                    refused: true,
                    ..query.reply.clone()
                })
            || super::client_row_p23c::exchange(client, 4, 0).await?.0 != *modern
        {
            return Err(format!(
                "step=legacy-negative-control action={action} observed={refused:?}"
            )
            .into());
        }
    }
    evidence(&format!(
        "P23C_CHECK step=legacy-negative-control projected-tasks={}",
        tasks.len()
    ));
    Ok(())
}

async fn prepare_hunt_ground(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let open = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    };
    let basis = spatial_basis::current(client)?;
    let path = route(world, from, |at| {
        open(at)
            && [
                StepDirection::North,
                StepDirection::East,
                StepDirection::South,
                StepDirection::West,
            ]
            .into_iter()
            .filter_map(|direction| destination(world, at, direction))
            .any(|adjacent| {
                open(adjacent)
                    && client.world_entities().is_some_and(|entities| {
                        entities
                            .iter()
                            .all(|entity| entity.position != basis.position(adjacent))
                    })
            })
    })
    .ok_or("step=hunt-ground no open adjacent fixture cell")?;
    walk(client, world, &path).await
}

async fn credit_one_kill(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    creature: &str,
) -> TestResult {
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let response = request_spawn(session, creature.to_owned())
        .await
        .map_err(|error| format!("step=fixture-hunt-input creature={creature} observed={error}"))?;
    let target = parse_spawned_entity(&response)?;
    evidence(&format!(
        "P23C_FIXTURE creature={creature} response={response}"
    ));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    tokio::time::timeout(Duration::from_secs(5), async {
        while !client.world_entities().is_some_and(|entities| {
            entities
                .get(&target)
                .is_some_and(|entry| entry.kind == oteryn_dev_client::EntityKind::Creature)
        }) {
            client.service_liveness(Duration::from_millis(100)).await?;
        }
        TestResult::Ok(())
    })
    .await
    .map_err(|_| "step=hunt-target exact fixture creature not visible")??;
    let attack = client.session_mut().attack_target(Some(&target)).await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        || client
            .session_mut()
            .combat_state()
            .is_none_or(|state| state.target != Some(target))
    {
        return Err(format!("step=hunt-attack target={target:?} observed={attack:?}").into());
    }
    let death = committed_hunt_death(
        client,
        clients,
        Some(clients.characters[0]),
        creature,
        target,
        deadline,
    )
    .await?;
    if death.shared || !death.awards.contains_key(&clients.characters[0]) {
        return Err(format!("step=hunt-death target={target:?} missing solo kill award").into());
    }
    evidence(&format!(
        "P23C_CHECK step=exact-kill creature={creature} target={target:?}"
    ));
    Ok(())
}

fn verify_settlement(before: &wire::full::Reply, after: &wire::full::Reply) -> TestResult {
    let expected_period = before
        .period
        .checked_add(7)
        .ok_or("weekly period overflow")?;
    if after.period != expected_period {
        return Err(format!(
            "step=weekly-settlement observed-period={} need=committed-reset-after-{} pending-points={}",
            after.period, before.period, before.weekly.pending_points
        ).into());
    }
    let expected = before
        .points
        .checked_add(before.weekly.pending_points)
        .ok_or("weekly balance overflow")?;
    let expected_seals = before
        .soulseals
        .checked_add(before.weekly.pending_soulseals)
        .ok_or("weekly soulseal balance overflow")?;
    if before.weekly.pending_points == 0
        || after.refusal != 0
        || after.points != expected
        || after.soulseals != expected_seals
        || after.bounty_points != before.bounty_points
        || after.weekly.started
        || after.weekly.pending_points != 0
        || after.weekly.pending_soulseals != 0
    {
        return Err(format!(
            "step=weekly-settlement expected-points={expected} observed={after:?}"
        )
        .into());
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(LogoutResult::Accepted) {
        return Err(format!("step=task-board-logout observed={reply:?}").into());
    }
    Ok(())
}

fn token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_:.,=/".contains(character) {
                character
            } else {
                '_'
            }
        })
        .collect()
}
