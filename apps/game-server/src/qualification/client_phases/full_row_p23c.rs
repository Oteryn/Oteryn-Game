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
    let legacy = command(&mut client, 0, 0).await?;
    if legacy.status != CommandStatus::Accepted
        || legacy.reply.refused
        || !legacy.reply.tasks.is_empty()
    {
        return Err(format!("step=legacy-query observed={legacy:?}").into());
    }
    let (initial, _) = exchange(&mut client, 4, 0).await?;
    if initial.refusal != 0 || initial.weekly.started || initial.bounty.active.is_some() {
        return Err(format!("step=modern-query observed={initial:?}").into());
    }
    for action in [1, 2] {
        let refused = command(&mut client, action, 1).await?;
        if refused.status != CommandStatus::Accepted
            || refused.reply
                != (wire::TaskBoardReply {
                    refused: true,
                    ..legacy.reply.clone()
                })
            || exchange(&mut client, 4, 0).await?.0 != initial
        {
            return Err(format!(
                "step=legacy-negative-control action={action} observed={refused:?}"
            )
            .into());
        }
    }

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
        || task.current != 0
        || task.required == 0
    {
        return Err(format!("step=weekly-start observed={weekly:?}").into());
    }

    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    for current in 1..=task.required {
        travel_to_spawn(&mut client, world, &metadata.spawns, &task.race).await?;
        client_hunt::hunt(&mut client).await?;
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
    let (early, _) = exchange(&mut client, 9, 0).await?;
    if early
        != (wire::full::Reply {
            refusal: 2,
            ..selection.clone()
        })
    {
        return Err(format!("step=bounty-early-claim observed={early:?}").into());
    }
    for current in 1..=offer.required {
        travel_to_spawn(&mut client, world, &metadata.spawns, &offer.race).await?;
        client_hunt::hunt(&mut client).await?;
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
        || claimed.weekly.pending_points < 25
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
    verify_settlement(&claimed, &persisted)
}

fn verify_settlement(before: &wire::full::Reply, after: &wire::full::Reply) -> TestResult {
    if after.period <= before.period {
        return Err(format!(
            "step=weekly-settlement observed-period={} need=committed-reset-after-{} pending-points={}",
            after.period, before.period, before.weekly.pending_points
        ).into());
    }
    let expected = before
        .points
        .checked_add(before.weekly.pending_points)
        .ok_or("weekly balance overflow")?;
    if before.weekly.pending_points == 0
        || after.refusal != 0
        || after.points != expected
        || after.bounty_points != before.bounty_points
        || after.weekly.started
        || after.weekly.pending_points != 0
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
