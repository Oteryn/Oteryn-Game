use super::*;
#[test]
fn task_board_shows_server_tasks_only() {
    let panel = TaskBoardPanel::default();
    assert!(!board_text(&panel, Tab::Bounty).contains("Invented race"));
    assert!(board_text(&panel, Tab::Weekly).contains("Refresh"));
}
#[test]
fn task_board_actions_are_bound_and_confirm_paid() {
    assert!(paid(&TaskBoardControl::Request(FullIntent::Reroll)));
    assert!(!paid(&TaskBoardControl::Request(FullIntent::ClaimBounty)));
}
#[test]
fn soulseals_and_weekly_results_show_server_data() {
    let panel = TaskBoardPanel::default();
    assert!(board_text(&panel, Tab::Soulseals).contains("No fight catalogue loaded"));
    assert!(!board_text(&panel, Tab::Results).contains("Weekly rewards received"));
}
#[test]
fn task_board_actions_rejected_when_session_replaced() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut incoming, _) = crate::play::play_channel();
    let panel = link.task_board_panel();
    assert!(!link.task_board_action_bound(&panel, TaskBoardControl::Request(FullIntent::Refresh)));
    assert_eq!(link.task_board_panel(), panel);
    assert!(incoming.try_recv().is_err());
    Ok(())
}
