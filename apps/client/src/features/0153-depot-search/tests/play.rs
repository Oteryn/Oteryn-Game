use crate::play::play_channel;
#[test]
fn depot_search_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    outbox.depot_search_initialize(true);
    link.depot_search_request(vec![]);
    assert!(matches!(
        rx.try_recv(),
        Ok(crate::combat_input::PlayCommand::ServerMechanic(_))
    ));
}
