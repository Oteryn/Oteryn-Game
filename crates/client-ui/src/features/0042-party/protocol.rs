use super::{Intent, PartyCommand, PartyPanel, PartyResponse, party};

pub fn request(panel: &mut PartyPanel, intent: Intent) -> Option<PartyCommand> {
    let prepared = prepare(panel, intent);
    match prepared {
        Ok(command) => {
            panel.awaiting_shared_feedback = command.shared_setting().is_some();
            panel.server_feedback = None;
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
        return Err(tr!("Parties are unavailable in this session"));
    }
    if panel.pending.is_some() {
        return Err(tr!("Waiting for the server"));
    }
    if matches!(intent, Intent::Kick | Intent::PassLeadership) && !panel.can_manage_member() {
        return Err(tr!("Only the party leader can manage another member"));
    }
    let intent = match intent {
        Intent::Refresh => return Ok(PartyCommand::query()),
        Intent::Invite(entity) => party::PartyIntent::Invite(entity),
        Intent::Join => party::PartyIntent::Accept(
            panel
                .selected_invitation
                .ok_or(tr!("Select an invitation first"))?,
        ),
        Intent::Leave => party::PartyIntent::Leave,
        Intent::Kick => party::PartyIntent::Kick(
            panel
                .selected_member
                .ok_or(tr!("Select a party member first"))?,
        ),
        Intent::PassLeadership => party::PartyIntent::PassLeadership(
            panel
                .selected_member
                .ok_or(tr!("Select a party member first"))?,
        ),
        Intent::ToggleSharedExperience => {
            if !panel.chat_selected {
                return Err(tr!(
                    "Shared experience requires chat support in this session"
                ));
            }
            let state = panel
                .state
                .as_ref()
                .ok_or(tr!("Refresh the party before changing shared experience"))?;
            return PartyCommand::shared_experience(!state.shared_xp_enabled)
                .map_err(|_| tr!("The shared experience request could not be encoded"));
        }
    };
    PartyCommand::intent(intent).map_err(|_| tr!("The party request could not be encoded"))
}

pub fn apply_result(panel: &mut PartyPanel, response: &PartyResponse) -> bool {
    if panel.pending.take().is_none() {
        return false;
    }
    let operation = party::decode_party_result(&response.operation).ok();
    if !matches!(
        response.status,
        oteryn_client_runtime::party::CommandStatus::Accepted
            | oteryn_client_runtime::party::CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some(tr!("The server refused the party command"));
    } else if operation.is_none() {
        panel.feedback = Some(tr!("The server returned an invalid party result"));
    } else if let Some(result) = operation
        .as_ref()
        .filter(|value| value.disposition != party::PartyDisposition::Ok)
    {
        panel.feedback = Some(disposition_text(result.disposition));
    } else {
        panel.feedback = Some(tr!("The server confirmed the party request"));
    }
    let Some(state) = response
        .view
        .as_deref()
        .and_then(|bytes| party::decode_party_result(bytes).ok())
    else {
        if operation
            .as_ref()
            .is_some_and(|result| result.disposition == party::PartyDisposition::Ok)
            && matches!(
                response.status,
                oteryn_client_runtime::party::CommandStatus::Accepted
                    | oteryn_client_runtime::party::CommandStatus::DuplicateReplay
            )
        {
            panel.feedback = Some(tr!("The current party view is unavailable; refresh again"));
        }
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
    panel.reconcile_list();
    panel.received_at = Some(std::time::Instant::now());
    true
}

fn disposition_text(value: party::PartyDisposition) -> &'static str {
    match value {
        party::PartyDisposition::Ok => tr!("The server confirmed the party request"),
        party::PartyDisposition::NotLeader => {
            tr!("The server refused: only the party leader can do that")
        }
        party::PartyDisposition::AlreadyInParty => {
            tr!("The server refused: that player is already in a party")
        }
        party::PartyDisposition::NotInvited => {
            tr!("The server refused: that invitation is no longer available")
        }
        party::PartyDisposition::NotVisible => {
            tr!("The server refused: that player is not visible")
        }
        party::PartyDisposition::LogoutBlocked => {
            tr!("The server refused: leaving is blocked during logout")
        }
        party::PartyDisposition::Unavailable => {
            tr!("The server refused: the party service is unavailable")
        }
        party::PartyDisposition::Rejected => tr!("The server refused the party request"),
        party::PartyDisposition::NotMember => {
            tr!("The server refused: that player is not a party member")
        }
    }
}

/// The server sends detailed XP refusals on the existing Game private-chat feedback path.
pub fn push_feedback(panel: &mut PartyPanel, line: &oteryn_client_runtime::chat::ChatLine) {
    if panel.awaiting_shared_feedback
        && let oteryn_client_runtime::chat::ChatLine::Private { speaker_name, text } = line
        && speaker_name == "Game"
        && text.to_ascii_lowercase().contains("shared experience")
    {
        panel.server_feedback = Some(text.clone());
        panel.awaiting_shared_feedback = false;
    }
}
