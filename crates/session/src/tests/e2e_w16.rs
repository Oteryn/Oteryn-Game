//! W16 client replay: the Summer 2026 NPC Wayland Smythers answers `hi`, `danger`, `name` and
//! `bye`. The captured `STATE_DOMAIN_NPC_CONVERSATION` deltas reach `Session::apply_state_delta`
//! (`domain_dispatch.rs`) through the real wire frame and surface as
//! `SessionEvent::NpcConversation`; a malformed delta is refused and publishes no event.
use super::*;
use crate::tests::npc_conversation_replay::{built, captured, replay};

const NPC: [u8; 16] = [0x16; 16];
const REPLIES: [(&str, &str); 4] = [
    ("hi", "Wayland Smythers, in service of science"),
    ("danger", "humble messenger"),
    ("name", "Wayland Smythers"),
    ("bye", "do be careful"),
];

#[test]
fn e2e_w16_wayland_smythers_dialogue_replay() -> Result<(), BoxError> {
    let frames = if let Some(dir) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        captured("w16", std::path::Path::new(&dir))?
    } else {
        let open = Some((NPC, 2));
        vec![
            built(
                "hi",
                (41, 4),
                open,
                &["Wayland Smythers, in service of science, at your service."],
            )?,
            built(
                "danger",
                (43, 5),
                open,
                &["Danger? Only for a humble messenger like you."],
            )?,
            built("name", (45, 6), open, &["I am Wayland Smythers."])?,
            built("bye", (47, 7), open, &["Farewell, and do be careful."])?,
        ]
    };
    assert_eq!(
        frames
            .iter()
            .map(|frame| frame.step.as_str())
            .collect::<Vec<_>>(),
        REPLIES.iter().map(|(step, _)| *step).collect::<Vec<_>>()
    );
    for (frame, (step, needle)) in frames.iter().zip(REPLIES) {
        assert!(
            frame.lines.iter().any(|line| line.contains(needle)),
            "W16 {step} reply lacks {needle:?}: {:?}",
            frame.lines
        );
    }
    replay("w16", frames)
}
