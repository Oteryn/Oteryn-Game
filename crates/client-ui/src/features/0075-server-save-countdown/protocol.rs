use crate::tutorial_notices::NoticePanel;
use oteryn_client_runtime::tutorial_notices::{ServerSaveCountdown, server_save};
pub fn apply(panel: &mut NoticePanel, value: ServerSaveCountdown) {
    panel.apply(server_save(value));
}
#[test]
fn server_save_replaces_previous_announcement() {
    let mut panel = NoticePanel::default();
    apply(
        &mut panel,
        ServerSaveCountdown {
            ending_epoch: 1,
            minutes_remaining: 5,
        },
    );
    apply(
        &mut panel,
        ServerSaveCountdown {
            ending_epoch: 1,
            minutes_remaining: 1,
        },
    );
    assert_eq!(panel.save_minutes, Some(1));
}
