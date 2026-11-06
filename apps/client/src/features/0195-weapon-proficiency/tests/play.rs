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

#[test]
fn nx8_proficiency_catalogue_can_be_requested_from_native_panel() {
    let mut panel = WeaponProficiencyPanel {
        selected: true,
        ..Default::default()
    };
    let command = native_controls::submit(&mut panel, 109, ["", "", "", "", "", ""], false);
    assert_eq!(
        command.map(|command| (command.kind, command.payload)),
        Some((660, vec![0, 0, 0, 0]))
    );
}

#[test]
fn nx8_proficiency_catalogue_displays_experience_full_perks_and_pages() {
    use oteryn_client_runtime::weapon_proficiency::{
        ProficiencyCatalogueEntry, ProficiencyCatalogueLevel, ProficiencyCataloguePage,
    };
    let mut panel = WeaponProficiencyPanel {
        selected: true,
        ..Default::default()
    };
    assert!(native_controls::submit(&mut panel, 109, [""; 6], false).is_some());
    let page = ProficiencyCataloguePage {
        offset: 0,
        total: 9,
        entries: (0..8)
            .map(|i| ProficiencyCatalogueEntry {
                definition: i + 1,
                weapon: format!("oteryn:item.weapon{i}"),
                name: format!("Weapon {i}"),
                experience: 2345,
                thresholds: [500, 1500, 3000, 7000, 15000, 30000, 60000, 120000, 240000],
                levels: vec![ProficiencyCatalogueLevel {
                    selected: Some(1),
                    perks: vec![
                        "Attack damage +1".into(),
                        "Defence +2".into(),
                        "Sword +1".into(),
                    ],
                }],
            })
            .collect(),
    };
    let result = GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: page.encode().unwrap_or_default(),
    };
    assert!(apply_result(&mut panel, &result));
    let text = native_controls::display(&panel);
    for expected in [
        "2345 XP",
        "Weapons 1-8 of 9",
        "Attack damage +1",
        "Defence +2",
        "Sword +1",
        "Weapon 7",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    let next = native_controls::submit(&mut panel, 110, [""; 6], false);
    assert_eq!(
        next.map(|command| command.payload),
        Some(8_u32.to_be_bytes().to_vec())
    );
    panel.pending = false;
    let previous = native_controls::submit(&mut panel, 111, [""; 6], false);
    assert_eq!(
        previous.map(|command| command.payload),
        Some(0_u32.to_be_bytes().to_vec())
    );
    assert!(!apply_result(
        &mut panel,
        &GatedCommandResult {
            payload: vec![0; 10],
            ..result
        }
    ));
    assert!(panel.feedback.is_some());
}
