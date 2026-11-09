use super::*;

pub fn soulseals_text(panel: &TaskBoardPanel) -> String {
    use std::fmt::Write;
    let p = &panel.fights;
    let mut text = String::from("Soulseal fights\r\n");
    if let Some(reply) = &p.reply {
        if let Some(balance) = reply.balance {
            let _ = write!(text, "Balance: {balance} Soulseals\r\n");
        }
        let _ = write!(
            text,
            "Solo fight | Level {} required | {} second limit\r\nStand on the first Soulpit entrance tile, alive and idle.\r\n",
            reply.rules.required_level, reply.rules.time_limit_seconds
        );
        for fight in &reply.entries {
            let _ = write!(
                text,
                "{}{} — {} Soulseals ({} stars)\r\n",
                if p.selected == Some(fight.race) {
                    "> "
                } else {
                    ""
                },
                fight.name,
                fight.cost,
                fight.stars
            );
        }
        if reply.authorization.is_some()
            && p.received_at.is_some_and(|at| {
                at.elapsed()
                    < std::time::Duration::from_millis(u64::from(reply.rules.authorization_ms))
            })
        {
            text.push_str("Entrance authorization is valid briefly; entry consumes it once.\r\n");
        } else if reply.status == task_board::soulpit::Status::Catalogue {
            text.push_str("Refresh at the entrance to authorize a fight.\r\n");
        }
        if !reply.reason.is_empty() {
            let _ = write!(text, "{}\r\n", reply.reason);
        }
    } else {
        text.push_str("No fight catalogue loaded. Refresh to load the confirmed balance and available fights.\r\n");
    }
    if let Some(weekly) = &panel.weekly {
        let _ = write!(
            text,
            "Pending weekly rewards: {} Soulseals\r\n",
            weekly.weekly.pending_soulseals
        );
    }
    if p.pending.is_some() {
        text.push_str("Waiting for the server\r\n");
    }
    text.push_str(&p.feedback);
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
