use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use oteryn_client_ui::cyclopedia_character::{
    CyclopediaCharacterPanel, Intent, apply_result, request,
};
#[test]
fn controls_encode_exact_request_and_reply_and_refusal_are_visible() {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        offset: 258,
        limit: 20,
        ..Default::default()
    };
    let c = request(&mut p, Intent::Tab).unwrap_or_else(|| unreachable!());
    assert_eq!(c.payload, [1, 2, 1, 20]);
    let ok = GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: vec![1, 0],
    };
    assert!(apply_result(&mut p, &ok));
    assert!(p.result.is_some());
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
fn native_character_navigation_and_input_gates() {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    for tab in 1..=10 {
        p.pending = false;
        let c = native_controls::submit(&mut p, 100 + tab, ["258", "20", ""]);
        assert_eq!(c.map(|c| c.payload), Some(vec![tab as u8, 2, 1, 20]));
        assert!(native_controls::submit(&mut p, 100 + tab, ["258", "20", ""]).is_none());
    }
    p.pending = false;
    assert!(native_controls::submit(&mut p, 101, ["0", "0", ""]).is_none());
    assert!(!p.pending);
    assert!(p.feedback.is_some());
    assert_eq!(
        native_controls::submit(&mut p, 111, ["bad", "bad", "bad"]).map(|c| c.kind),
        Some(387)
    );
    p.result = Some("Badges: Hero".into());
    assert!(native_controls::display(&p).contains("Badges: Hero"));
}

#[test]
fn native_character_target_and_page_validation() {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    for values in [
        ["65536", "20", ""],
        ["0", "51", ""],
        ["0", "20", "invalid"],
        ["0", "20", "é"],
    ] {
        assert!(native_controls::submit(&mut p, 101, values).is_none());
        assert!(!p.pending);
    }
    let command = native_controls::submit(
        &mut p,
        107,
        ["0", "1", "00112233-4455-6677-8899-aabbccddeeff"],
    );
    assert_eq!(
        command.map(|c| c.payload),
        Some(vec![
            7, 0, 0, 1, 0, 17, 34, 51, 68, 85, 102, 119, 136, 153, 170, 187, 204, 221, 238, 255
        ])
    );
    p.pending = false;
    p.selected = false;
    assert!(native_controls::submit(&mut p, 111, ["0", "20", ""]).is_none());
}
