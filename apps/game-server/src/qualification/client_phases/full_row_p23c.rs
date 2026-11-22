//! P23c: check the legacy Prey Hunting Tasks query, selection and claim against
//! the served board. The production owner retired both mutations and exposes
//! balances through the legacy query. The replacement weekly and bounty board is exercised by P04.

use super::*;
use oteryn_dev_client::{DevClientSession, client_runtime::task_board};
use oteryn_protocol_oteryn::{logout::LogoutResult, task_board as wire};

const ROW: &str = "P23c";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let failure = match result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => Some("step-1/2 timeout; no hunting-task reply observed".to_owned()),
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
    let offered = initial.reply.tasks.len();
    let task = 1;
    // The legacy fixed board numbers its first task 1. Even with no offers in
    // the projection, exercise both recognized command codes and check custody.
    for (action, name) in [(1, "select"), (2, "claim")] {
        let reply = command(&mut client, action, task).await?;
        if reply.status != CommandStatus::Accepted
            || !reply.reply.refused
            || !same_durable_view(&initial.reply, &reply.reply)
        {
            return Err(format!("step=hunting-task-{name}: expected inert retired action; baseline={initial:?} observed={reply:?}; source:crates/gameplay/dispatch/src/gameplay_transport/task_board.rs:82").into());
        }
        evidence(&format!(
            "P23C_CHECK action={name} observed=retired-action-refused points={}",
            reply.reply.points
        ));
    }

    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xc4, 1).await?;
    let persisted = command(&mut fresh, 0, 0).await?;
    logout(&mut fresh).await?;
    if fresh.join_snapshot().game_session_id == session
        || persisted.status != CommandStatus::Accepted
        || persisted.reply.refused
        || !same_durable_view(&initial.reply, &persisted.reply)
    {
        return Err(format!(
            "step=hunting-task-durability observed={persisted:?} baseline={initial:?}"
        )
        .into());
    }

    Err(format!("step=hunting-task-reward observed=options-{offered},select-refused,claim-refused need=Prey-Hunting-Task-options-kill-credit-and-reward-claim; source:crates/gameplay/dispatch/src/gameplay_transport/task_board.rs:82").into())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(LogoutResult::Accepted) {
        return Err(format!("logout-refused;observed={reply:?}").into());
    }
    Ok(())
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

#[cfg(test)]
mod tests {
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
