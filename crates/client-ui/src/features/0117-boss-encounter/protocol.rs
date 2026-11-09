use super::BossEncounterPanel;
use oteryn_client_runtime::{
    boss_encounter,
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Catalogue,
    CatalogueNext,
    Refresh,
    Next,
    Enter,
    Defeated,
    Timeout,
    HazardQuery,
    HazardSelect,
    HazardLower,
    HazardRaise,
    SlotsQuery,
    SlotEquip,
    SlotRemove,
    Rewards,
    RewardsNext,
}
pub fn request(panel: &mut BossEncounterPanel, intent: Intent) -> Option<FeatureCommand> {
    if !panel.selected {
        panel.feedback = Some("Boss encounter status is unavailable in this session");
        return None;
    }
    if panel.pending {
        return None;
    }
    if matches!(intent, Intent::Catalogue | Intent::CatalogueNext) {
        let query = if intent == Intent::CatalogueNext {
            let mut query = panel.catalogue_query.clone()?;
            query.after = panel.catalogue.as_ref()?.next?;
            query
        } else {
            boss_encounter::catalogue::Query {
                after: 0,
                category: panel.browser_category.server_filter(),
                search: panel.browser_search.trim().into(),
            }
        };
        let Some(payload) = query.encode() else {
            panel.feedback = Some("Search must fit 48 UTF-8 bytes");
            return None;
        };
        panel.pending = true;
        panel.pending_kind = boss_encounter::COMMAND_TYPE_BOSS_COOLDOWNS;
        panel.pending_catalogue = Some(query);
        panel.feedback = None;
        return Some(FeatureCommand {
            capability: 117,
            kind: 133,
            payload,
        });
    }
    panel.pending_catalogue = None;
    if matches!(intent, Intent::Enter | Intent::Defeated | Intent::Timeout) {
        let Ok(payload) = boss_encounter::encode_boss_key(&panel.fields[0]) else {
            panel.feedback = Some("Enter a valid boss key");
            return None;
        };
        let kind = match intent {
            Intent::Enter => boss_encounter::COMMAND_TYPE_BOSS_LEVER,
            Intent::Defeated => boss_encounter::COMMAND_TYPE_BOSS_DEFEATED,
            Intent::Timeout => boss_encounter::COMMAND_TYPE_BOSS_TIMEOUT,
            _ => return None,
        };
        panel.pending = true;
        panel.pending_kind = kind;
        panel.feedback = None;
        return Some(FeatureCommand {
            capability: boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
            kind,
            payload,
        });
    }
    if !matches!(intent, Intent::Refresh | Intent::Next) {
        let Some(command) = super::auxiliary::command(panel, intent) else {
            panel.feedback = Some("Enter valid boss, slot (1-2), or hazard level (1-12)");
            return None;
        };
        panel.pending = true;
        panel.pending_kind = command.kind;
        panel.feedback = None;
        return Some(command);
    }
    let after = match intent {
        Intent::Refresh => "",
        Intent::Next => {
            let page = panel.snapshot.as_ref()?;
            match (&page.cooldown_next, &page.progress_next) {
                (Some(a), Some(b)) => a.min(b).as_str(),
                (Some(a), None) | (None, Some(a)) => a.as_str(),
                (None, None) => return None,
            }
        }
        _ => return None,
    };
    let command = boss_encounter::query_taints(after)?;
    panel.pending = true;
    panel.pending_kind = command.kind;
    panel.feedback = None;
    Some(command)
}
pub fn apply_result(panel: &mut BossEncounterPanel, result: &GatedCommandResult) -> bool {
    panel.pending = false;
    if let Some(query) = panel.pending_catalogue.take() {
        if !matches!(
            result.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) {
            panel.feedback = Some("The server refused the boss catalogue query");
            return false;
        }
        let Some(page) = boss_encounter::catalogue::Page::decode(&result.payload) else {
            panel.feedback = Some("Invalid boss catalogue reply");
            return false;
        };
        if page.next.is_some_and(|next| next <= query.after)
            || page.entries.iter().any(|e| {
                query.category != 0 && e.category != query.category
                    || !e.name.to_lowercase().contains(&query.search.to_lowercase())
            })
        {
            panel.feedback = Some("Mismatched boss catalogue reply");
            return false;
        }
        if !page.entries.iter().any(|e| e.key == panel.fields[0]) {
            panel.fields[0] = page
                .entries
                .first()
                .map_or_else(String::new, |e| e.key.clone());
        }
        panel.catalogue = Some(page);
        panel.catalogue_query = Some(query);
        panel.feedback = None;
        return true;
    }
    if matches!(
        panel.pending_kind,
        boss_encounter::COMMAND_TYPE_BOSS_LEVER
            | boss_encounter::COMMAND_TYPE_BOSS_DEFEATED
            | boss_encounter::COMMAND_TYPE_BOSS_TIMEOUT
    ) {
        let accepted = matches!(
            result.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        );
        if !accepted && result.status != CommandStatus::Rejected {
            panel.feedback = Some("The server refused the boss request");
            return false;
        }
        if !accepted && result.payload.is_empty() {
            panel.feedback = Some("The server refused the boss request");
            return false;
        }
        let Ok(reply) = boss_encounter::decode_boss_activity_reply(&result.payload) else {
            panel.feedback = Some("The server returned invalid boss activity data");
            return false;
        };
        match reply {
            boss_encounter::BossActivityReply::Accepted {
                deadline_unix_ms, ..
            } if accepted => {
                panel.summary = match deadline_unix_ms {
                    Some(deadline) => format!("Boss entry accepted. Deadline: {deadline}"),
                    None => "Boss return accepted".into(),
                };
                panel.feedback = None;
            }
            boss_encounter::BossActivityReply::Refused(reason) => {
                panel.summary = format!("Boss request refused: {reason:?}");
                panel.feedback = Some("The server refused the boss request");
            }
            _ => {
                panel.feedback = Some("The server refused the boss request");
                return false;
            }
        }
        return true;
    }
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some("The server refused the boss status query");
        return false;
    }
    if panel.pending_kind != boss_encounter::COMMAND_TYPE_BOSS_COOLDOWNS {
        let Some(summary) = super::auxiliary::summary(panel, &result.payload) else {
            panel.feedback = Some("The server returned invalid boss data");
            return false;
        };
        panel.summary = summary;
        panel.feedback = None;
        return true;
    }
    let Some(snapshot) = boss_encounter::decode(&result.payload) else {
        panel.feedback = Some("The server returned invalid boss status");
        return false;
    };
    panel.snapshot = Some(snapshot);
    panel.cooldown_received_at = Some(std::time::Instant::now());
    panel.feedback = None;
    true
}
