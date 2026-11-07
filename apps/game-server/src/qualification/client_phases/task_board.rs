use super::*;
pub(super) async fn task_board(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::task_board, client_ui::task_board as ui};
    if !client.selected_capabilities().contains(&72) {
        evidence(
            "mechanic=task_board skipped=capability_72_not_selected source=crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities.rs:126",
        );
        return Ok(());
    }
    let mut panel = ui::TaskBoardPanel {
        selected: true,
        ..Default::default()
    };
    let request = ui::weekly_request(&mut panel, None).ok_or("weekly query")?;
    let reply = task_board::send_full(client.session_mut(), &request).await?;
    if reply.refusal != 0 || reply.weekly.started {
        return Err(format!("new character weekly state: {reply:?}").into());
    }
    ui::apply_weekly(&mut panel, reply);
    let request = ui::weekly_request(&mut panel, Some(0)).ok_or("weekly start")?;
    let reply = task_board::send_full(client.session_mut(), &request).await?;
    if reply.refusal != 0 {
        let reason = if reply.refusal == 3 {
            "character_below_weekly_minimum_level_8"
        } else {
            "weekly_owner_refused_start"
        };
        evidence(&format!(
            "mechanic=task_board skipped={reason} refusal={} source=crates/gameplay/quests/src/quest/task_board_full.rs:620",
            reply.refusal
        ));
        return Ok(());
    }
    ui::apply_weekly(&mut panel, reply);
    let request = ui::weekly_request(&mut panel, None).ok_or("weekly current query")?;
    ui::apply_weekly(
        &mut panel,
        task_board::send_full(client.session_mut(), &request).await?,
    );
    let current = panel.weekly.ok_or("weekly reducer lost state")?;
    if current.refusal != 0
        || !current.weekly.started
        || current.weekly.difficulty != 0
        || current.weekly.kills.is_empty()
    {
        return Err("weekly task start was not reflected by current query".into());
    }
    evidence(
        "mechanic=task_board weekly_start=committed current_query_started=true difficulty=0 kill_tasks=nonempty",
    );
    Ok(())
}
