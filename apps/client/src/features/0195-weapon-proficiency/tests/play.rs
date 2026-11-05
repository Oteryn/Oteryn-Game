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

#[path = "../native.rs"]
mod native_controls;
#[test]
fn native_proficiency_actions_and_input_gates() {
    let mut p = WeaponProficiencyPanel {
        selected: true,
        ..Default::default()
    };
    for (id, kind) in [
        (101, 378),
        (102, 379),
        (103, 380),
        (104, 392),
        (105, 393),
        (106, 394),
        (107, 394),
        (108, 395),
    ] {
        p.pending = false;
        let c = native_controls::submit(&mut p, id, ["258", "2", "1", "1", "9", "2"], false);
        assert_eq!(c.map(|c| c.kind), Some(kind));
        assert!(
            native_controls::submit(&mut p, id, ["258", "2", "1", "1", "9", "2"], false).is_none()
        );
    }
    p.pending = false;
    assert!(
        native_controls::submit(&mut p, 101, ["bad", "2", "1", "1", "9", "2"], false).is_none()
    );
    assert!(!p.pending);
    assert!(p.feedback.is_some());
    p.result = Some("Selection accepted".into());
    assert!(native_controls::display(&p).contains("Selection accepted"));
}

#[test]
fn native_proficiency_rejects_invalid_fields_and_clears_perks() {
    let mut p = WeaponProficiencyPanel {
        selected: true,
        ..Default::default()
    };
    for (id, values) in [
        (101, ["0", "2", "1", "1", "9", "2"]),
        (101, ["258", "7", "1", "1", "9", "2"]),
        (102, ["258", "2", "1", "0", "9", "2"]),
        (103, ["258", "2", "1", "1", "0", "2"]),
        (106, ["258", "2", "1", "1", "9", "3"]),
    ] {
        assert!(native_controls::submit(&mut p, id, values, false).is_none());
        assert!(!p.pending);
    }
    let command = native_controls::submit(&mut p, 101, ["258", "2", "1", "1", "9", "2"], true);
    assert_eq!(command.map(|c| c.payload), Some(vec![0, 0, 1, 2, 2, 255]));
    p.pending = false;
    p.selected = false;
    assert!(
        native_controls::submit(&mut p, 101, ["258", "2", "1", "1", "9", "2"], false).is_none()
    );
}
