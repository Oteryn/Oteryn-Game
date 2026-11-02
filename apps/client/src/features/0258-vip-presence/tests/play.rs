#[test]
fn panel_reports_selection() {
    let panel = oteryn_client_ui::vip_presence::VipPresencePanel {
        selected: true,
        ..Default::default()
    };
    assert!(!oteryn_client_ui::vip_presence::panel_text(&panel).contains("unavailable"));
}
