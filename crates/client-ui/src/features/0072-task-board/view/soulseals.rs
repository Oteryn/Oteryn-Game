use super::*;

pub fn soulseals_text(panel: &TaskBoardPanel) -> String {
    let mut text = String::from("Soul Seals fight dialog\r\n");
    if let Some(reply) = &panel.weekly {
        text.push_str(&format!(
            "Balance: {} Soulseals\r\nPending weekly rewards: {} Soulseals\r\n",
            reply.soulseals, reply.weekly.pending_soulseals
        ));
    } else {
        text.push_str("Refresh to load the confirmed balance\r\n");
    }
    text.push_str("No fight catalogue is supplied by this server protocol. Encounter selection is unavailable.");
    text
}

pub fn soulseals_key(events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
    use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
    events.iter().any(|e| matches!(e, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 8 && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Alt)
            && modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fight_dialog_never_fabricates_a_balance_or_available_encounters() {
        let panel = TaskBoardPanel::default();
        let text = soulseals_text(&panel);
        assert!(text.contains("Refresh"));
        assert!(text.contains("No fight catalogue"));
        assert!(!text.contains("Balance: 0"));
    }
}
