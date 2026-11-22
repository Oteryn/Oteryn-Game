use crate::play::play_channel;
use oteryn_client_ui::chat::Mode;
type Error = Box<dyn std::error::Error + Send + Sync>;

#[test]
fn native_second_send_preserves_unsent_composer() -> Result<(), Error> {
    let (link, mut commands, outbox) = play_channel();
    let mut mailbox = crate::play::mailbox::lock(&outbox.0);
    mailbox.chat.selected = true;
    mailbox.chat.text = "first".into();
    assert!(super::submit_composer(
        &mut mailbox,
        &link.commands,
        Mode::Say,
        ""
    ));
    assert!(matches!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::ChatPublic(_))
    ));
    mailbox.chat.text = "unsent second".into();
    assert!(!super::submit_composer(
        &mut mailbox,
        &link.commands,
        Mode::Say,
        ""
    ));
    assert_eq!(mailbox.chat.text, "unsent second");
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn native_composer_enter_queues_the_existing_send_action() {
    let mut actions = Vec::new();
    assert!(!super::route_keyboard(false, false, &mut actions));
    assert!(actions.is_empty());
    assert!(super::route_keyboard(true, true, &mut actions));
    assert!(actions.is_empty());
    assert!(super::route_keyboard(false, true, &mut actions));
    assert_eq!(actions, vec![105]);
}
