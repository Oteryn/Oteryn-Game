#[test]
fn violation_reason_picker_wraps_named_reasons_and_invalidates_review() {
    let mut panel = oteryn_client_ui::player_report::Panel::default();
    panel.fields[0] = "0".into();
    panel.violation_reviewed = Some(vec![1]);
    panel.cycle_reason(false);
    assert_eq!(panel.fields[0], "20");
    assert!(oteryn_client_ui::player_report::panel_text(&panel).contains("Service agreement"));
    assert!(panel.violation_reviewed.is_none());
    panel.cycle_reason(true);
    assert_eq!(panel.fields[0], "0");
    assert!(oteryn_client_ui::player_report::panel_text(&panel).contains("Inappropriate name"));
}
