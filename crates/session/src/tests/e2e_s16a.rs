//! S16a client replay: Oressa's first-vocation dialogue reaches the client as
//! `STATE_DOMAIN_NPC_CONVERSATION` deltas, consumed by `Session::apply_state_delta`
//! (`domain_dispatch.rs`) and exposed as `SessionEvent::NpcConversation`.
use super::*;
use crate::tests::npc_conversation_replay::{built, captured, replay};

const NPC: [u8; 16] = [0x16; 16];

#[test]
fn e2e_s16a_oressa_vocation_dialogue_replay() -> Result<(), BoxError> {
    let frames = if let Some(dir) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        captured("s16a", std::path::Path::new(&dir))?
    } else {
        let open = Some((NPC, 3));
        vec![
            built(
                "hi",
                (41, 4),
                open,
                &["Welcome. Ask about a vocation when you are ready to choose."],
            )?,
            built(
                "druid",
                (43, 5),
                open,
                &["Do you wish to become a druid? Say yes to confirm, or no to reconsider."],
            )?,
            built(
                "yes",
                (45, 6),
                open,
                &["Reach level 8 before making your first vocation choice."],
            )?,
        ]
    };
    assert!(
        frames
            .iter()
            .any(|frame| frame.lines == ["Reach level 8 before making your first vocation choice."]),
        "S16a replay lacks the refused second choice"
    );
    replay("s16a", frames)
}
