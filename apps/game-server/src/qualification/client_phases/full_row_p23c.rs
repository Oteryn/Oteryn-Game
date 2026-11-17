//! P23c — Prey Hunting Tasks: reward options, claim and points.
//!
//! Concrete player scenario, derived from the P23c audit/matrix row and the named
//! Task Board owner (`quest/task_board.rs`):
//! 1. Log in with a node-boot character and query the hunting-task board; expect
//!    an accepted reply containing the offered creature tasks and current points.
//! 2. Select one offered task with the native player command; expect an accepted,
//!    non-refused reply whose selected task changes while kills and points do not.
//! 3. Repeat that select while a task is active; expect a refused reply and no
//!    selected-task, kill, point, or claim-time delta.
//! 4. Walk to the selected creature's authored spawn and kill the required count
//!    through ordinary combat; after each kill, query and expect the kill delta,
//!    with points and claim time unchanged before claim.
//! 5. Send the task claim command; expect selection/kills to clear, points to rise
//!    by 25, and that option's claim timestamp to become nonzero.
//! 6. Claim the same option again; expect refusal with no state delta.
//! 7. Log out, freshly admit the same character, and query; expect the points and
//!    claim timestamp to persist and the completed task to remain unselected.
//!
//! The journey starts at step 1 and sends step 2 even on servers which expose the
//! board but retire hunting-task selection. Such a runtime refusal is reported as
//! FAIL with the observed reply and the server capability needed for steps 3–7;
//! it is never converted into a source-only or pre-decided verdict.

use super::*;
use oteryn_dev_client::{DevClientSession, client_runtime::task_board};
use oteryn_protocol_oteryn::{logout::LogoutResult, task_board as wire};

const ROW: &str = "P23c";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let failure = match result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => Some("step-1/2 timeout; no hunting-task reply observed".to_owned()),
    };
    match failure {
        None => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario={ROW}.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Some(error) => {
            evidence(&format!("P23C_GAP {error}"));
            evidence(&format!(
                "SEAM_EVIDENCE e2e_all scenario={ROW}.full status=FAIL seconds={:.3} evidence={}",
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
    if initial.status != CommandStatus::Accepted
        || initial.reply.refused
        || initial.reply.tasks.is_empty()
    {
        return Err(format!("step-1 query-invalid;observed={initial:?};need=task-options").into());
    }
    evidence(&format!(
        "P23C_CHECK step=1 status=PASS observed=options:{};points:{}",
        initial.reply.tasks.len(),
        initial.reply.points
    ));

    let selected = command(&mut client, 1, 1).await?;
    if selected.status != CommandStatus::Accepted
        || selected.reply.refused
        || selected.reply.selected != 1
        || selected.reply.kills != 0
        || selected.reply.points != initial.reply.points
    {
        if !same_durable_view(&initial.reply, &selected.reply) {
            return Err(format!(
                "step-2 select-refused-with-delta;observed={selected:?};need=atomic-hunting-task-selection"
            )
            .into());
        }
        // Re-check the refusal across a real logout and fresh admission. This is
        // durable negative-path evidence, not a substitute for the missing claim.
        logout(&mut client).await?;
        drop(client);
        let mut fresh = connect_character(clients, 0xc4, 1).await?;
        let persisted = command(&mut fresh, 0, 0).await?;
        logout(&mut fresh).await?;
        if persisted.status != CommandStatus::Accepted
            || persisted.reply.refused
            || !same_durable_view(&initial.reply, &persisted.reply)
        {
            return Err(format!(
                "step-2 refused-select-mutated-after-relog;select={selected:?};fresh={persisted:?}"
            )
            .into());
        }
        return Err(format!(
            "step-2 select-refused;observed=status:{:?},refused:{},selected:{},kills:{},points:{};need=player-hunting-task-select-kill-claim-owner",
            selected.status,
            selected.reply.refused,
            selected.reply.selected,
            selected.reply.kills,
            selected.reply.points
        )
        .into());
    }

    // A supported server reaches here. Keeping later lifecycle work behind the
    // successful selection prevents a refusal from being mistaken for progress.
    Err("step-3 lifecycle-not-reached-after-supported-select;need=complete-combat-route".into())
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
    fn evidence_token_has_no_whitespace() {
        assert_eq!(
            token("step 2: Rejected { x: 1 }"),
            "step_2:_Rejected___x:_1__"
        );
    }
}
