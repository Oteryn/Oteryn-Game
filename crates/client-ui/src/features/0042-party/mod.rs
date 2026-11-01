//! Server-confirmed party membership and invitations.
pub use oteryn_client_runtime::party::{EntityRef, PartyCommand, PartyResponse, party};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Invite(EntityRef),
    Join,
    Leave,
    PassLeadership,
    Kick,
    ToggleSharedExperience,
    Refresh,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PartyPanel {
    pub selected: bool,
    pub open: bool,
    pub state: Option<party::PartyResult>,
    pub selected_member: Option<u64>,
    pub selected_invitation: Option<u64>,
    pub pending: Option<PartyCommand>,
    pub feedback: Option<&'static str>,
}

pub fn request(panel: &mut PartyPanel, intent: Intent) -> Option<PartyCommand> {
    let prepared = prepare(panel, intent);
    match prepared {
        Ok(command) => {
            panel.pending = Some(command.clone());
            panel.feedback = None;
            Some(command)
        }
        Err(message) => {
            panel.feedback = Some(message);
            None
        }
    }
}

fn prepare(panel: &PartyPanel, intent: Intent) -> Result<PartyCommand, &'static str> {
    if !panel.selected {
        return Err("Parties are unavailable in this session");
    }
    if panel.pending.is_some() {
        return Err("Waiting for the server");
    }
    let intent = match intent {
        Intent::Refresh => return Ok(PartyCommand::query()),
        Intent::Invite(entity) => party::PartyIntent::Invite(entity),
        Intent::Join => party::PartyIntent::Accept(
            panel
                .selected_invitation
                .ok_or("Select an invitation first")?,
        ),
        Intent::Leave => party::PartyIntent::Leave,
        Intent::Kick => {
            party::PartyIntent::Kick(panel.selected_member.ok_or("Select a party member first")?)
        }
        Intent::PassLeadership => party::PartyIntent::PassLeadership(
            panel.selected_member.ok_or("Select a party member first")?,
        ),
        Intent::ToggleSharedExperience => {
            return Err("Shared experience can only be changed through party chat");
        }
    };
    PartyCommand::intent(intent).map_err(|_| "The party request could not be encoded")
}

pub fn apply_result(panel: &mut PartyPanel, response: &PartyResponse) -> bool {
    if panel.pending.take().is_none() {
        return false;
    }
    let operation = party::decode_party_result(&response.operation).ok();
    if let Some(result) = operation
        .as_ref()
        .filter(|value| value.disposition != party::PartyDisposition::Ok)
    {
        panel.feedback = Some(disposition_text(result.disposition));
    } else {
        panel.feedback = Some("The server confirmed the party request");
    }
    let Some(state) = response
        .view
        .as_deref()
        .and_then(|bytes| party::decode_party_result(bytes).ok())
    else {
        panel.feedback = Some("The current party view is unavailable; refresh again");
        return false;
    };
    if state.disposition != party::PartyDisposition::Ok {
        panel.feedback = Some(disposition_text(state.disposition));
        return false;
    }
    panel.selected_member = panel
        .selected_member
        .filter(|handle| state.members.iter().any(|entry| entry.handle == *handle));
    panel.selected_invitation = panel.selected_invitation.filter(|handle| {
        state
            .invitations
            .iter()
            .any(|entry| entry.handle == *handle)
    });
    panel.state = Some(state);
    true
}

fn disposition_text(value: party::PartyDisposition) -> &'static str {
    match value {
        party::PartyDisposition::Ok => "The server confirmed the party request",
        party::PartyDisposition::NotLeader => {
            "The server refused: only the party leader can do that"
        }
        party::PartyDisposition::AlreadyInParty => {
            "The server refused: that player is already in a party"
        }
        party::PartyDisposition::NotInvited => {
            "The server refused: that invitation is no longer available"
        }
        party::PartyDisposition::NotVisible => "The server refused: that player is not visible",
        party::PartyDisposition::LogoutBlocked => {
            "The server refused: leaving is blocked during logout"
        }
        party::PartyDisposition::Unavailable => {
            "The server refused: the party service is unavailable"
        }
        party::PartyDisposition::Rejected => "The server refused the party request",
        party::PartyDisposition::NotMember => {
            "The server refused: that player is not a party member"
        }
    }
}

#[must_use]
pub fn panel_text(panel: &PartyPanel) -> String {
    let mut text = String::from("Party\n");
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
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(disposition: party::PartyDisposition) -> party::PartyResult {
        party::PartyResult {
            disposition,
            shared_xp_enabled: true,
            members: vec![party::PartyEntry {
                handle: 7,
                name: "Leader".into(),
                leader: true,
                expires_at: 0,
            }],
            invitations: Vec::new(),
        }
    }

    #[test]
    fn request_uses_party_codec_and_requires_selection() -> Result<(), &'static str> {
        let mut panel = PartyPanel {
            selected: true,
            selected_member: Some(7),
            ..PartyPanel::default()
        };
        let command = request(&mut panel, Intent::Kick).ok_or("selected member makes a command")?;
        assert_eq!(
            party::decode_party_intent(&command.payload),
            Ok(party::PartyIntent::Kick(7))
        );
        Ok(())
    }

    #[test]
    fn result_applies_view_and_surfaces_refusal() -> Result<(), party::PartyWireError> {
        let mut panel = PartyPanel {
            selected: true,
            pending: Some(PartyCommand::query()),
            ..PartyPanel::default()
        };
        let response = PartyResponse {
            status: oteryn_client_runtime::party::CommandStatus::Accepted,
            operation: party::encode_party_result(&result(party::PartyDisposition::NotLeader))?,
            view: Some(party::encode_party_result(&result(
                party::PartyDisposition::Ok,
            ))?),
        };
        assert!(apply_result(&mut panel, &response));
        assert!(panel_text(&panel).contains("only the party leader"));
        assert!(panel_text(&panel).contains("Leader: Leader"));
        Ok(())
    }
}
