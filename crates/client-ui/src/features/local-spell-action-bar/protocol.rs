//! Movement spells use ordinary local speech and the server's current spell owner.
use crate::chat_public_channels::ChatPanel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementSpell {
    LevitateUp,
    LevitateDown,
    MagicRope,
}

pub fn request(panel: &mut ChatPanel, spell: MovementSpell) -> Option<&'static str> {
    if !panel.speech_selected || panel.pending {
        return None;
    }
    panel.pending = true;
    panel.feedback = None;
    Some(match spell {
        MovementSpell::LevitateUp => tr!("exani hur \"up"),
        MovementSpell::LevitateDown => tr!("exani hur \"down"),
        MovementSpell::MagicRope => tr!("exani tera"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_public_channels::apply_result;
    use oteryn_client_runtime::chat_public_channels::{ChatDisposition, ChatOutcome};

    #[test]
    fn movement_spells_use_local_speech_and_wait_for_the_server() {
        for (spell, words) in [
            (MovementSpell::LevitateUp, "exani hur \"up"),
            (MovementSpell::LevitateDown, "exani hur \"down"),
            (MovementSpell::MagicRope, "exani tera"),
        ] {
            let mut panel = ChatPanel::default();
            assert_eq!(request(&mut panel, spell), None);
            panel.speech_selected = true;
            assert_eq!(request(&mut panel, spell), Some(words));
            assert!(panel.pending);
            assert_eq!(request(&mut panel, spell), None);
            apply_result(
                &mut panel,
                &ChatOutcome {
                    command_id: 1,
                    status: oteryn_client_runtime::CommandStatus::Accepted,
                    disposition: ChatDisposition::Rejected,
                    wait_seconds: 0,
                    result_server_sequence: 1,
                },
            );
            assert!(!panel.pending);
            assert!(panel.feedback.is_some());
        }
    }
}
