use super::*;
#[test]
fn cyclopedia_character_window_shows_server_data_only() {
    let mut panel = Panel::default();
    assert!(body(&panel).contains("No server character view yet"));
    panel.result = Some("server-owned character facts".into());
    assert!(body(&panel).contains("server-owned character facts"));
    panel.items.page = Some(oteryn_client_runtime::cyclopedia_character::items::Page {
        offset: 0,
        total: 1,
        rows: vec![oteryn_client_runtime::cyclopedia_character::items::Row {
            definition: "confirmed sword".into(),
            quantities: [1, 2, 3, 4],
        }],
    });
    panel.items.open = true;
    let text = body(&panel);
    assert!(text.contains("confirmed sword"));
    assert!(text.contains("[1, 2, 3, 4]"));
}
#[test]
fn cyclopedia_character_window_registered() {
    assert!(include_str!("../mod.rs").contains("cyclopedia_character::install(app)"));
    assert!(include_str!("../../ui.rs").contains("shell.cyclopedia_character"));
    assert!(include_str!("../../ui.rs").contains("cyclopedia_character::toggle"));
}
