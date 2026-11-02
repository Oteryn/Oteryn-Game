use super::*;
use oteryn_client_runtime::cl25a_protocol::{
    VipPresenceBatch, VipPresenceEntry, VipPresenceStatus,
};

#[test]
fn pushed_vip_presence_is_visible_with_online_and_offline_state() {
    let mut panel = VipPresencePanel {
        selected: true,
        ..Default::default()
    };
    replace(
        &mut panel,
        VipPresenceBatch {
            entries: vec![
                VipPresenceEntry {
                    entry: 1,
                    name: "Online Friend".into(),
                    status: VipPresenceStatus::Online,
                    announce: true,
                    confirmed_for_ms: 1_000,
                },
                VipPresenceEntry {
                    entry: 2,
                    name: "Offline Friend".into(),
                    status: VipPresenceStatus::Offline,
                    announce: false,
                    confirmed_for_ms: 0,
                },
            ],
        },
    );
    let text = panel_text(&panel);
    assert!(text.contains("Online Friend — Online"));
    assert!(text.contains("Offline Friend — Offline"));
}
