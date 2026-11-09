//! Soulseal choices are received from the server; only the server commits a debit.
use super::TaskBoardPanel;
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
use oteryn_client_runtime::task_board::task_board::soulpit as wire;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FightPanel {
    pub reply: Option<wire::Reply>,
    pub selected: Option<u16>,
    pub pending: Option<wire::Request>,
    pub received_at: Option<std::time::Instant>,
    pub feedback: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FightIntent {
    Catalogue { after: u16 },
    EnterSelected,
}
pub fn request(panel: &mut TaskBoardPanel, intent: FightIntent) -> Option<FeatureCommand> {
    let p = &mut panel.fights;
    if !panel.selected {
        p.feedback = "Soulseal fights unavailable in this session".into();
        return None;
    }
    if p.pending.is_some() {
        p.feedback = "Waiting for the server".into();
        return None;
    }
    let request = match intent {
        FightIntent::Catalogue { after } => wire::Request::Catalogue { after },
        FightIntent::EnterSelected => {
            let Some(reply) = &p.reply else {
                p.feedback = "Refresh the fight catalogue".into();
                return None;
            };
            let Some(fight) = reply.entries.iter().find(|f| Some(f.race) == p.selected) else {
                p.feedback = "Select a confirmed fight".into();
                return None;
            };
            let Some(authorization) = reply.authorization else {
                p.feedback = "Refresh for a new entrance authorization".into();
                return None;
            };
            if p.received_at.is_none_or(|at| {
                at.elapsed()
                    >= std::time::Duration::from_millis(u64::from(reply.rules.authorization_ms))
            }) {
                p.feedback = "Entrance authorization expired; refresh the catalogue".into();
                return None;
            }
            if reply.balance.is_none_or(|b| b < u64::from(fight.cost)) {
                p.feedback = "Not enough confirmed Soulseals; refresh the balance".into();
                return None;
            }
            wire::Request::Enter {
                authorization,
                race: fight.race,
            }
        }
    };
    let payload = request.encode().ok()?;
    p.pending = Some(request);
    p.feedback.clear();
    if let Some(reply) = &mut p.reply {
        reply.authorization = None;
    }
    Some(FeatureCommand {
        capability: 72,
        kind: wire::COMMAND_TYPE,
        payload,
    })
}
pub fn apply(panel: &mut TaskBoardPanel, command: &FeatureCommand, result: &GatedCommandResult) {
    let p = &mut panel.fights;
    if command.capability != 72
        || command.kind != wire::COMMAND_TYPE
        || wire::Request::decode(&command.payload).ok().as_ref() != p.pending.as_ref()
        || p.pending.is_none()
    {
        return;
    }
    let request = p.pending.take();
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.feedback = "The server refused the fight request; refresh to retry".into();
        return;
    }
    let Ok(reply) = wire::Reply::decode(&result.payload) else {
        p.feedback = "Invalid fight reply".into();
        return;
    };
    if let Some(wire::Request::Catalogue { after }) = request
        && reply.status == wire::Status::Catalogue
        && reply.entries.iter().any(|f| f.race <= after)
    {
        p.feedback = "Invalid catalogue order".into();
        return;
    }
    if reply.status == wire::Status::Catalogue {
        p.selected = reply.entries.first().map(|f| f.race);
        p.received_at = Some(std::time::Instant::now());
    } else {
        p.selected = None;
        p.received_at = None;
    }
    p.feedback = if reply.status == wire::Status::Started {
        "Fight entry committed by the server".into()
    } else {
        reply.reason.clone()
    };
    p.reply = Some(reply);
}
pub fn select(panel: &mut TaskBoardPanel, race: u16) -> bool {
    let p = &mut panel.fights;
    if !panel.selected
        || p.pending.is_some()
        || !p.reply.as_ref().is_some_and(|r| {
            r.status == wire::Status::Catalogue && r.entries.iter().any(|f| f.race == race)
        })
    {
        return false;
    }
    p.selected = Some(race);
    true
}
