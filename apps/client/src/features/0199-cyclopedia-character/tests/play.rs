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
