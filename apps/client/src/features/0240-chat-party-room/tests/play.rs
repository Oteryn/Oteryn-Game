#[test]
fn panel_reports_selection() {
    let panel = oteryn_client_ui::chat_party_room::ChatPartyRoomPanel {
        selected: true,
        ..Default::default()
    };
    assert!(!oteryn_client_ui::chat_party_room::panel_text(&panel).contains("unavailable"));
}
