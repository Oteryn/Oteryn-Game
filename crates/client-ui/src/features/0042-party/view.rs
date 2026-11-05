use super::PartyPanel;

#[must_use]
pub fn panel_text(panel: &PartyPanel) -> String {
    let mut text = String::from("Party\n");
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
            "Shared experience: enabled\n"
        } else {
            "Shared experience: disabled\n"
        });
        if state.members.is_empty() {
            text.push_str("Members: none\n");
        }
        for member in &state.members {
            text.push_str(if member.leader {
                "Leader: "
            } else {
                "Member: "
            });
            text.push_str(&member.name);
            text.push('\n');
        }
        for invitation in &state.invitations {
            text.push_str("Invitation: ");
            text.push_str(&invitation.name);
            text.push('\n');
        }
    } else {
        text.push_str("Refresh to load the current party.\n");
    }
    text
}
