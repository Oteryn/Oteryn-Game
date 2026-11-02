use super::*;
pub(super) async fn task_board(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::task_board, client_ui::task_board as ui};
    // Task Board needs a bundle-world offer and native gameplay catalogue; the entry room has neither.
    // Keep the phase strict whenever the serving node selects its mechanic.
    if !client.selected_capabilities().contains(&72) {
        evidence("mechanic=task_board skipped=entry_room_owner_not_composed");
        return Ok(());
    }
    let mut panel = ui::TaskBoardPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("task board request")?;
    let reply = task_board::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if panel.pending || panel.state.is_none() {
        return Err("task board reducer did not load state".into());
    }
    evidence("mechanic=task_board client_codec=real reducer=real state=loaded");
    Ok(())
}
