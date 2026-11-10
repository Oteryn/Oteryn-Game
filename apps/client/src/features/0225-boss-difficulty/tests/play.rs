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
    assert_eq!(c.payload, b"\x08\x04\x12\x0doteryn:dragon");
    let payload = encode_reply(&Reply::Window {
        context: [1; 16],
        boss_key: "oteryn:dragon".into(),
        boss_creature_key: "oteryn:dragon".into(),
        content_revision: "r1".into(),
        selected_difficulty: 2,
        personal_highest: 0,
        unlocked_max: 1,
        maximum_difficulty: 25,
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

#[cfg(not(windows))]
#[path = "../native.rs"]
mod native;

#[cfg(not(windows))]
#[test]
fn boss_difficulty_native_controls_validate_and_send() {
    let mut p = BossDifficultyPanel {
        selected: true,
        ..Default::default()
    };
    assert!(native::submit(&mut p, Intent::Open, "oteryn:dragon", "", "2").is_some());
    assert!(p.pending);
    assert!(!native::enabled(&p));
    assert!(native::submit(&mut p, Intent::Open, "other", "", "2").is_none());
    assert_eq!(p.boss_key, "oteryn:dragon");
    p.pending = false;
    assert!(
        native::submit(
            &mut p,
            Intent::Preview,
            "oteryn:dragon",
            "éxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            "2"
        )
        .is_none()
    );
    assert!(
        native::submit(
            &mut p,
            Intent::Preview,
            "oteryn:dragon",
            "01010101010101010101010101010101",
            "bad"
        )
        .is_none()
    );
    let command = native::submit(
        &mut p,
        Intent::Preview,
        "oteryn:dragon",
        "01010101010101010101010101010101",
        "2",
    );
    assert_eq!(command.map(|c| (c.capability, c.kind)), Some((225, 431)));
    assert_eq!(p.context, Some([1; 16]));
    apply_result(
        &mut p,
        &GatedCommandResult {
            command_id: 2,
            status: CommandStatus::Rejected,
            server_sequence: 2,
            payload: vec![],
        },
    );
    assert!(!p.pending);
    assert!(oteryn_client_ui::boss_difficulty::panel_text(&p).contains("refused"));
    assert!(native::enabled(&p));
    p.selected = false;
    assert!(!native::enabled(&p));
}

#[cfg(not(windows))]
#[test]
fn boss_difficulty_native_actions_use_admitted_commands() {
    use crate::combat_input::PlayCommand;
    use crate::play::play_channel;
    use oteryn_client_runtime::boss_difficulty::{Request, decode_request};
    let (link, mut rx, _) = play_channel();
    let mut p = BossDifficultyPanel {
        selected: true,
        ..Default::default()
    };
    for (intent, expected) in [
        (
            Intent::Open,
            Request::OpenWithProgress {
                boss_key: "oteryn:dragon".into(),
            },
        ),
        (
            Intent::Preview,
            Request::Preview {
                context: [1; 16],
                difficulty: 2,
            },
        ),
        (
            Intent::Start,
            Request::Start {
                context: [1; 16],
                difficulty: 2,
            },
        ),
        (Intent::Cancel, Request::Cancel { context: [1; 16] }),
    ] {
        p.pending = false;
        let command = native::submit(
            &mut p,
            intent,
            "oteryn:dragon",
            "01010101010101010101010101010101",
            "2",
        );
        assert!(command.is_some());
        if let Some(command) = command {
            assert!(link.commands.send(PlayCommand::Feature(command)).is_ok());
        }
        let received = rx.try_recv();
        assert!(
            matches!(&received, Ok(PlayCommand::Feature(c)) if c.kind == 431 && c.capability == 225 && decode_request(&c.payload) == Ok(expected.clone()))
        );
    }
    for (context, difficulty) in [
        ("00000000000000000000000000000000", "2"),
        ("01010101010101010101010101010101", "29"),
    ] {
        p.pending = false;
        assert!(native::submit(&mut p, Intent::Start, "", context, difficulty).is_none());
        assert!(!p.pending);
    }
}

#[path = "context.rs"]
mod context;
