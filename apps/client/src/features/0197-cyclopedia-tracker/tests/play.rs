use oteryn_client_ui::cyclopedia_tracker::{CyclopediaTrackerPanel, Intent, request};
#[test]
fn cyclopedia_tracker_maps_supported_command() {
    let mut panel = CyclopediaTrackerPanel {
        selected: true,
        ..Default::default()
    };
    let command = request(
        &mut panel,
        Intent {
            command: 383,
            payload: Vec::new(),
        },
    );
    assert_eq!(
        command.map(|value| (value.capability, value.kind)),
        Some((197, 383))
    );
}
