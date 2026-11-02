use oteryn_client_runtime::{
    boss_difficulty::{Reply, encode_reply},
    raw_feature::{CommandStatus, GatedCommandResult},
};
use oteryn_client_ui::boss_difficulty::{BossDifficultyPanel, Intent, apply_result, request};
#[test]
fn controls_encode_exact_request_and_reply_and_refusal_are_visible() {
    let mut p = BossDifficultyPanel {
        selected: true,
        boss_key: "oteryn:dragon".into(),
        ..Default::default()
    };
    let c = request(&mut p, Intent::Open).unwrap_or_else(|| unreachable!());
    assert_eq!(c.payload, b"\x08\x03\x12\x0doteryn:dragon");
    let payload = encode_reply(&Reply::Window {
        context: [1; 16],
        boss_key: "oteryn:dragon".into(),
        content_revision: "r1".into(),
        selected_difficulty: 2,
    })
    .unwrap_or_default();
    let ok = GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    };
    assert!(apply_result(&mut p, &ok));
    assert_eq!(p.context, Some([1; 16]));
    let refused = GatedCommandResult {
        status: CommandStatus::Rejected,
        ..ok
    };
    assert!(!apply_result(&mut p, &refused));
    assert!(p.feedback.is_some());
}
