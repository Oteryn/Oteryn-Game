use crate::play::play_channel;
#[test]
fn hotkey_equip_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    outbox.hotkey_equip_initialize(true);
    link.hotkey_equip_request(vec![]);
    assert!(matches!(
        rx.try_recv(),
        Ok(crate::combat_input::PlayCommand::ServerMechanic(_))
    ));
}
