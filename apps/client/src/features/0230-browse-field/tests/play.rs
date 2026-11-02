use crate::play::play_channel;
#[test]
fn browse_field_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    outbox.browse_field_initialize(true);
    link.browse_field_request(vec![]);
    assert!(matches!(
        rx.try_recv(),
        Ok(crate::combat_input::PlayCommand::ServerMechanic(_))
    ));
}
