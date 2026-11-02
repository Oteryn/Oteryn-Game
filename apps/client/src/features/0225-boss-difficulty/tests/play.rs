use oteryn_client_ui::boss_difficulty::{BossDifficultyPanel, Intent, request};
#[test]
fn boss_difficulty_maps_supported_command() {
    let mut panel = BossDifficultyPanel {
        selected: true,
        ..Default::default()
    };
    let command = request(
        &mut panel,
        Intent {
            command: 431,
            payload: Vec::new(),
        },
    );
    assert_eq!(
        command.map(|value| (value.capability, value.kind)),
        Some((225, 431))
    );
}
