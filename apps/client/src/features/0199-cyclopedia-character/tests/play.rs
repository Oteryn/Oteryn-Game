use oteryn_client_ui::cyclopedia_character::{CyclopediaCharacterPanel, Intent, request};
#[test]
fn cyclopedia_character_maps_supported_command() {
    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    let command = request(
        &mut panel,
        Intent {
            command: 387,
            payload: Vec::new(),
        },
    );
    assert_eq!(
        command.map(|value| (value.capability, value.kind)),
        Some((199, 387))
    );
}
