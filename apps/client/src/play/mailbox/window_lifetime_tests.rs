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

#[test]
fn shared_frontend_is_session_owned_and_keeps_chat_state_live() {
    let (owner, _commands, _outbox) = play_channel();
    assert!(!owner.uses_shared_frontend());
    owner.use_shared_frontend();
    let window = PlayLink {
        owns_session: false,
        mailbox: std::sync::Arc::clone(&owner.mailbox),
        commands: owner.commands.clone(),
    };
    assert!(window.uses_shared_frontend());
    owner.open_chat();
    assert!(owner.chat_panel().open);
    let (other, _commands, _outbox) = play_channel();
    assert!(!other.uses_shared_frontend());
}

#[test]
fn shared_frontend_inventory_shortcut_obeys_session_shutdown() {
    let (owner, mut commands, outbox) = play_channel();
    owner.use_shared_frontend();
    assert!(owner.activate_shortcut("inventory"));
    assert!(owner.inventory_panel().open);
    assert!(commands.try_recv().is_err());
    outbox.inventory_end();
    assert!(owner.activate_shortcut("inventory"));
    assert!(!owner.inventory_panel().open);
    assert!(commands.try_recv().is_err());
}

#[test]
fn shared_frontend_chat_sends_existing_command_without_local_echo() {
    let (owner, mut commands, outbox) = play_channel();
    owner.use_shared_frontend();
    outbox.chat_initialize(true);
    owner.open_chat();
    owner.chat_text("hello from shared UI".into());
    owner.chat_request(oteryn_client_ui::chat::Mode::Say, "");
    assert!(matches!(
        commands.try_recv(),
        Ok(PlayCommand::ChatPublic(oteryn_session::ChatIntent::Say { text, .. }))
            if text == "hello from shared UI"
    ));
    assert!(owner.chat_panel().lines.is_empty());
    assert!(owner.chat_panel().pending);
}
