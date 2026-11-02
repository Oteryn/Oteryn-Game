use oteryn_client_ui::weapon_proficiency::{Intent, WeaponProficiencyPanel, request};
#[test]
fn weapon_proficiency_maps_supported_command() {
    let mut panel = WeaponProficiencyPanel {
        selected: true,
        ..Default::default()
    };
    let command = request(
        &mut panel,
        Intent {
            command: 378,
            payload: Vec::new(),
        },
    );
    assert_eq!(
        command.map(|value| (value.capability, value.kind)),
        Some((195, 378))
    );
}
