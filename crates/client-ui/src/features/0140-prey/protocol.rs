use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    ListReroll,
    Select,
    SelectAny,
    BonusReroll,
    SetOption,
}

pub fn request(panel: &mut PreyPanel, intent: Intent) -> Option<PreyCommand> {
    if !panel.selected {
        panel.feedback = Some("Prey is unavailable in this session");
        return None;
    }
    if panel.pending {
        panel.feedback = Some("Waiting for the server");
        return None;
    }
    let request = match intent {
        Intent::Refresh => prey::Request::Query,
        Intent::ListReroll => prey::Request::ListReroll { slot: panel.slot },
        Intent::Select => prey::Request::Select {
            slot: panel.slot,
            index: panel.candidate,
        },
        Intent::SelectAny => prey::Request::SelectAny {
            slot: panel.slot,
            creature: panel.creature.clone(),
        },
        Intent::BonusReroll => prey::Request::BonusReroll { slot: panel.slot },
        Intent::SetOption => prey::Request::SetOption {
            slot: panel.slot,
            option: panel.option,
        },
    };
    if prey::encode_request(&request).is_err() {
        panel.feedback = Some("Choose a valid slot, candidate, creature, or option");
        return None;
    }
    panel.pending = true;
    panel.feedback = None;
    Some(PreyCommand::new(request))
}

pub fn apply_result(panel: &mut PreyPanel, response: &PreyResponse) -> bool {
    panel.pending = false;
    if !matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some("The server refused the Prey action");
        return false;
    }
    let Some(reply) = &response.reply else {
        panel.feedback = Some("The server returned no Prey state");
        return false;
    };
    panel.reply = Some(reply.clone());
    panel.feedback = match reply.status {
        0 => None,
        prey::STATUS_CONTENT_UNAVAILABLE => {
            Some("The selected creature's loot is not available on this server")
        }
        _ => Some("The server refused the Prey action"),
    };
    reply.status == 0
}
