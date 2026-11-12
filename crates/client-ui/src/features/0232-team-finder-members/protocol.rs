use super::*;
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
fn request(
    panel: &mut Panel,
    command_type: u32,
    payload: Vec<u8>,
) -> Option<ServerMechanicCommand> {
    if !panel.selected || panel.pending.is_some() {
        return None;
    }
    panel.pending = Some(command_type);
    panel.feedback = tr!("Waiting for the server").into();
    Some(ServerMechanicCommand {
        capability: 232,
        command_type,
        payload,
    })
}
pub fn query(panel: &mut Panel, applicants: bool, next: bool) -> Option<ServerMechanicCommand> {
    let after = if next {
        if panel.applicants != applicants {
            return None;
        }
        Some(panel.next()?)
    } else {
        None
    };
    let payload = encode_query(if applicants {
        TeamFinderQuery::Applicants { after }
    } else {
        TeamFinderQuery::Browse { after }
    })
    .ok()?;
    let command = request(panel, 444, payload)?;
    panel.applicants = applicants;
    panel.selected_row = 0;
    panel.result = None;
    Some(command)
}
pub fn member(panel: &mut Panel, cancel: bool) -> Option<ServerMechanicCommand> {
    if panel.applicants {
        return None;
    }
    let leader = panel.handle()?;
    request(
        panel,
        445,
        encode_member_action(if cancel {
            MemberAction::Cancel { leader }
        } else {
            MemberAction::Request { leader }
        })
        .ok()?,
    )
}
pub fn leader(panel: &mut Panel, action: u8) -> Option<ServerMechanicCommand> {
    if !panel.applicants {
        return None;
    }
    let member = panel.handle()?;
    let action = match action {
        2 => LeaderAction::Invite { member },
        3 => LeaderAction::Accept { member },
        4 => LeaderAction::Deny { member },
        _ => return None,
    };
    request(panel, 446, encode_leader_action(action).ok()?)
}
pub fn apply_result(panel: &mut Panel, response: &ServerMechanicResponse) {
    let Some(kind) = panel.pending.take() else {
        return;
    };
    if matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        if kind == 444 {
            if let Ok(result) = decode_result(&response.payload)
                && matches!(result, TeamFinderResult::Applicants { .. }) == panel.applicants
            {
                panel.result = Some(result);
                panel.selected_row = 0;
                panel.feedback = tr!("Page received").into();
                return;
            }
        } else if response.payload.is_empty() {
            panel.result = None;
            panel.feedback = tr!("Decision confirmed; refresh to see current membership").into();
            return;
        }
    }
    panel.feedback = tr!("Team Finder request refused or invalid reply; refresh").into();
}
