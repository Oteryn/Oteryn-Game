//! P23c: qualify task selection, real kill credit, claim and relog when served.
//! A retired board must refuse both mutations without changing any balance.

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
    let mut client = connect_character(clients, 0xc3, 1).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_TASK_BOARD_V1)
    {
        return Err("step-1 capability-72-not-selected;need=hunting-task-command-owner".into());
    }

    let initial = command(&mut client, 0, 0).await?;
    if initial.status != CommandStatus::Accepted || initial.reply.refused {
        return Err(
            format!("step=hunting-task-query observed={initial:?} need=accepted-board").into(),
        );
    }
    let retired = initial.reply.tasks.is_empty();
    let expected = if retired {
        // Legacy task IDs are one-based even when the retired list is empty.
        for (action, name) in [(1, "select"), (2, "claim")] {
            let reply = command(&mut client, action, 1).await?;
            check_refusal(name, &initial.reply, &reply)?;
        }
        initial.reply
    } else {
        complete_task(&mut client, clients, &initial.reply).await?
    };

    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xc4, 1).await?;
    let persisted = command(&mut fresh, 0, 0).await?;
    logout(&mut fresh).await?;
    if fresh.join_snapshot().game_session_id == session
        || persisted.status != CommandStatus::Accepted
        || persisted.reply.refused
        || !same_durable_view(&expected, &persisted.reply)
    {
        return Err(format!(
            "step=hunting-task-durability observed={persisted:?} expected={expected:?}"
        )
        .into());
    }

    if retired {
        return Err("step=hunting-task-reward observed=options-0,select-refused,claim-refused need=Prey-Hunting-Task-options-kill-credit-and-reward-claim; source:crates/gameplay/dispatch/src/gameplay_transport/task_board.rs:82".into());
    }
    Ok(())
}

fn check_refusal(
    name: &str,
    expected: &wire::TaskBoardReply,
    reply: &task_board::TaskBoardResponse,
) -> TestResult {
    if reply.status != CommandStatus::Accepted
        || !reply.reply.refused
        || !same_durable_view(expected, &reply.reply)
    {
        return Err(
            format!("step=hunting-task-{name} need=inert-refusal observed={reply:?}").into(),
        );
    }
    Ok(())
}

async fn complete_task(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    initial: &wire::TaskBoardReply,
) -> TestResult<wire::TaskBoardReply> {
    let (index, task) = initial
        .tasks
        .iter()
        .enumerate()
        .find(|(_, task)| task.claimed_at_unix == 0 && task.required > 0)
        .ok_or("step=hunting-task-select need=offered-unclaimed-task")?;
    if initial.selected != 0 || initial.kills != 0 {
        return Err("step=hunting-task-select need=idle-task-board".into());
    }
    let slot = u8::try_from(index + 1)?;
    let selected = command(client, 1, slot).await?;
    let expected_selection = wire::TaskBoardReply {
        selected: slot,
        ..initial.clone()
    };
    if selected.status != CommandStatus::Accepted
        || selected.reply.refused
        || !same_durable_view(&expected_selection, &selected.reply)
    {
        return Err(format!(
            "step=hunting-task-select observed={selected:?} expected={expected_selection:?}"
        )
        .into());
    }
    let early = command(client, 2, slot).await?;
    check_refusal("premature-claim", &expected_selection, &early)?;

    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(420);
    for kills in 1..=task.required {
        let hunt = async {
            travel_to_spawn(client, world, &metadata.spawns, &task.race).await?;
            client_hunt::hunt(client).await
        };
        tokio::time::timeout_at(deadline, hunt)
            .await
            .map_err(|_| {
                format!(
                    "step=hunting-task-credit race={} needed={} credited={}",
                    task.race,
                    task.required,
                    kills - 1
                )
            })??;
        let updated = command(client, 0, 0).await?;
        let expected = wire::TaskBoardReply {
            kills,
            ..expected_selection.clone()
        };
        if updated.status != CommandStatus::Accepted
            || updated.reply.refused
            || !same_durable_view(&expected, &updated.reply)
        {
            return Err(format!(
                "step=hunting-task-credit race={} expected-kills={kills} observed={updated:?}",
                task.race
            )
            .into());
        }
    }
    let before_claim = wire::TaskBoardReply {
        kills: task.required,
        ..expected_selection
    };
    let claimed = command(client, 2, slot).await?;
    if claimed.status != CommandStatus::Accepted
        || !claim_applied(&before_claim, &claimed.reply, index)
    {
        return Err(format!("step=hunting-task-claim expected=cleared-selection-and-task-points observed={claimed:?}").into());
    }
    let duplicate = command(client, 2, slot).await?;
    check_refusal("duplicate-claim", &claimed.reply, &duplicate)?;
    Ok(claimed.reply)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    verdict::Harness::global().bounded_wait(deadline, "hunting-task-logout");
    loop {
        let reply = client.session_mut().logout().await?;
        match reply.result {
            Some(LogoutResult::Accepted) => return Ok(()),
            Some(LogoutResult::Busy | LogoutResult::InFight { .. })
                if tokio::time::Instant::now() < deadline =>
            {
                client.service_liveness(Duration::from_millis(100)).await?;
                client.take_events();
            }
            _ => return Err(format!("logout-refused;observed={reply:?}").into()),
        }
    }
}

fn same_durable_view(left: &wire::TaskBoardReply, right: &wire::TaskBoardReply) -> bool {
    left.selected == right.selected
        && left.kills == right.kills
        && left.points == right.points
        && left.bounty_points == right.bounty_points
        && left.talisman_levels == right.talisman_levels
        && left.talisman_bonuses == right.talisman_bonuses
        && left.talisman_next_costs == right.talisman_next_costs
        && left.tasks == right.tasks
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

fn claim_applied(
    before: &wire::TaskBoardReply,
    after: &wire::TaskBoardReply,
    index: usize,
) -> bool {
    let (Some(task), Some(claimed)) = (before.tasks.get(index), after.tasks.get(index)) else {
        return false;
    };
    if after.refused
        || usize::from(before.selected) != index + 1
        || before.kills < task.required
        || claimed.claimed_at_unix <= task.claimed_at_unix
    {
        return false;
    }
    let Some(points) = before
        .points
        .checked_add(oteryn_gameplay_quests::quest::task_board::TASK_POINTS)
    else {
        return false;
    };
    let mut expected = wire::TaskBoardReply {
        selected: 0,
        kills: 0,
        points,
        ..before.clone()
    };
    expected.tasks[index].claimed_at_unix = claimed.claimed_at_unix;
    same_durable_view(&expected, after)
}

#[cfg(test)]
mod tests {
    #[test]
    fn completed_claim_credits_points_once_and_clears_selection() {
        let ready = wire::TaskBoardReply {
            refused: false,
            selected: 1,
            kills: 2,
            points: 10,
            bounty_points: 0,
            talisman_levels: [0; 4],
            talisman_bonuses: [0; 4],
            talisman_next_costs: [5; 4],
            tasks: vec![wire::TaskBoardTask {
                race: "oteryn:creature.rat".into(),
                required: 2,
                claimed_at_unix: 0,
            }],
        };
        let mut claimed = ready.clone();
        claimed.selected = 0;
        claimed.kills = 0;
        claimed.tasks[0].claimed_at_unix = 1;
        claimed.points += 25;
        assert!(claim_applied(&ready, &claimed, 0));
        assert!(!claim_applied(&claimed, &claimed, 0));
        claimed.points += 1;
        assert!(!claim_applied(&ready, &claimed, 0));
    }

    use super::*;

    #[test]
    fn retired_action_cannot_change_either_currency_or_talisman_state() {
        let initial = wire::TaskBoardReply {
            refused: false,
            selected: 0,
            kills: 0,
            points: 25,
            bounty_points: 10,
            talisman_levels: [0; 4],
            talisman_bonuses: [0; 4],
            talisman_next_costs: [5; 4],
            tasks: Vec::new(),
        };
        let mut refused = initial.clone();
        refused.refused = true;
        assert!(same_durable_view(&initial, &refused));
        refused.bounty_points += 1;
        assert!(!same_durable_view(&initial, &refused));
        refused.bounty_points = initial.bounty_points;
        refused.talisman_levels[0] += 1;
        assert!(!same_durable_view(&initial, &refused));
    }

    #[test]
    fn evidence_token_has_no_whitespace() {
        assert_eq!(
            token("step 2: Rejected { x: 1 }"),
            "step_2:_Rejected___x:_1__"
        );
    }
}
