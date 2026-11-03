use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use oteryn_client_ui::weapon_proficiency::{Intent, WeaponProficiencyPanel, apply_result, request};
#[test]
fn controls_encode_exact_request_and_reply_and_refusal_are_visible() {
    let mut p = WeaponProficiencyPanel {
        selected: true,
        weapon: 258,
        level: 2,
        perk: 1,
        ..Default::default()
    };
    let c = request(&mut p, Intent::SelectPerk).unwrap_or_else(|| unreachable!());
    assert_eq!(c.payload, [0, 0, 1, 2, 2, 1]);
    let ok = GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: vec![0, 3, 0, 255, 2],
    };
    assert!(apply_result(&mut p, &ok));
    assert!(p.result.as_deref().is_some_and(|v| v.contains("Accepted")));
    let refused = GatedCommandResult {
        status: CommandStatus::Rejected,
        ..ok
    };
    assert!(!apply_result(&mut p, &refused));
    assert!(p.feedback.is_some());
}
