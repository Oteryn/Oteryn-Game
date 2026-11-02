use crate::play::play_channel;
#[test]
fn store_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    outbox.store_initialize(true);
    link.store_request(183, vec![]);
    assert!(matches!(
        rx.try_recv(),
        Ok(crate::combat_input::PlayCommand::ServerMechanic(_))
    ));
}
