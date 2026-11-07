use crate::play::play_channel;
use oteryn_client_ui::depot::{DepotPanel, Intent, request};
#[test]
fn depot_commands_require_authoritative_town_and_one_pending_request() {
    let mut p = DepotPanel {
        selected: true,
        ..Default::default()
    };
    assert!(request(&mut p, Intent::Withdraw(0)).is_none());
    assert!(
        request(
            &mut p,
            Intent::Browse {
                box_number: 1,
                page: 0
            }
        )
        .is_some()
    );
    assert!(
        request(
            &mut p,
            Intent::Browse {
                box_number: 2,
                page: 0
            }
        )
        .is_none()
    );
}
#[test]
fn depot_visible_panel_receives_authoritative_refusal() {
    let (link, _commands, outbox) = play_channel();
    outbox.depot_initialize(true);
    link.open_depot();
    assert!(link.depot_panel().pending);
    outbox.depot_result(None);
    assert!(!link.depot_panel().pending);
    assert!(link.depot_panel().feedback.contains("refused"));
}
