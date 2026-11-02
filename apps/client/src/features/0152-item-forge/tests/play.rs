use crate::play::play_channel;
#[test]
fn item_forge_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    outbox.item_forge_initialize(true);
    link.item_forge_request(vec![]);
    assert!(matches!(
        rx.try_recv(),
        Ok(crate::combat_input::PlayCommand::ServerMechanic(_))
    ));
}
