#[test]
fn o010_o037_window_buttons_toggle_real_panels_and_have_distinct_icons()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = crate::play::play_channel();
    outbox.spell_catalog_initialize(true, Some(Vec::new()))?;
    let mut settings = crate::play::SettingsState::new(None);
    settings
        .preferences
        .rebind("0287-actor-spell-catalog-open", "Alt+U")?;
    assert!(settings.queue_window("0287-actor-spell-catalog-open"));
    let (id, events) = settings
        .take_window_selection()
        .ok_or("missing menu selection")?;
    assert!(link.window_toggle(id, &events));
    assert!(link.window_open(id));
    assert!(link.spell_catalog_panel().open);
    assert!(link.window_toggle(id, &events));
    assert!(!link.window_open(id));
    assert!(!link.spell_catalog_panel().open);
    let icons: std::collections::BTreeSet<_> = crate::play::WINDOW_ACTIONS
        .iter()
        .map(|(id, _)| crate::play::window_icon(id))
        .collect();
    assert_eq!(icons.len(), crate::play::WINDOW_ACTIONS.len());
    assert!(!link.window_toggle("unknown", &events));
    outbox.logged_out();
    assert!(!link.window_open(id));
    Ok(())
}
