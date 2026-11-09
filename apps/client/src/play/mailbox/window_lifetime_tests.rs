use super::*;

#[test]
fn dialog_links_preserve_session_state_and_owner_drop_still_closes_open_dialogs() {
    let (owner, _commands, outbox) = play_channel();
    outbox.task_board_initialize(true);
    let window = PlayLink {
        owns_session: false,
        mailbox: std::sync::Arc::clone(&owner.mailbox),
        commands: owner.commands.clone(),
    };
    drop(window);
    assert!(owner.task_board_panel().selected);
    assert!(!lock(&outbox.0).wheel_shutdown);
    let window = PlayLink {
        owns_session: false,
        mailbox: std::sync::Arc::clone(&owner.mailbox),
        commands: owner.commands.clone(),
    };
    drop(owner);
    assert!(lock(&outbox.0).wheel_shutdown);
    assert!(!window.task_board_panel().selected);
    drop(window);
}
