use super::PartyPanel;

#[must_use]
pub fn panel_text(panel: &PartyPanel) -> String {
    let elapsed = panel.received_at.map_or(0, |at| {
        u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX)
    });
    panel_text_at(panel, elapsed)
}

/// Deterministic countdown rendering, also used by the native party window.
#[must_use]
pub fn panel_text_at(panel: &PartyPanel, elapsed_ms: u64) -> String {
    let mut text = String::from(tr!("Party\n"));
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
        text.push('\n');
    }
    if let Some(feedback) = &panel.server_feedback {
        text.push_str(feedback);
        text.push('\n');
    }
    if let Some(state) = &panel.state {
        text.push_str(if state.shared_xp_enabled {
            tr!("Shared experience: enabled\n")
        } else {
            tr!("Shared experience: disabled\n")
        });
        if state.members.is_empty() {
            text.push_str(tr!("Members: none\n"));
        }
        for member in &state.members {
            text.push_str(if member.leader {
                tr!("Leader: ")
            } else {
                tr!("Member: ")
            });
            text.push_str(&member.name);
            if let Some(durations) = member.party_spell_remaining_ms {
                for (name, remaining) in [
                    tr!("Train Party"),
                    tr!("Protect Party"),
                    tr!("Enchant Party"),
                    tr!("Heal Party"),
                    tr!("Enlighten Party"),
                ]
                .into_iter()
                .zip(durations)
                {
                    let remaining = u64::from(remaining).saturating_sub(elapsed_ms);
                    if remaining > 0 {
                        text.push_str(&lfmt!(
                            " | {name}: {}s",
                            remaining.div_ceil(1000),
                            name = name
                        ));
                    }
                }
            } else {
                text.push_str(tr!(" | effects unavailable"));
            }
            text.push('\n');
        }
        for invitation in &state.invitations {
            text.push_str(tr!("Invitation: "));
            text.push_str(&invitation.name);
            text.push('\n');
        }
    } else {
        text.push_str(tr!("Refresh to load the current party.\n"));
    }
    text
}
