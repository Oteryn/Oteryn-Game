use super::BossEncounterPanel;
use oteryn_client_runtime::{
    boss_encounter,
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Next,
}
pub fn request(panel: &mut BossEncounterPanel, intent: Intent) -> Option<FeatureCommand> {
    if !panel.selected {
        panel.feedback = Some("Boss encounter status is unavailable in this session");
        return None;
    }
    if panel.pending {
        return None;
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
    };
    let command = boss_encounter::query(after)?;
    panel.pending = true;
    panel.feedback = None;
    Some(command)
}
pub fn apply_result(panel: &mut BossEncounterPanel, result: &GatedCommandResult) -> bool {
    panel.pending = false;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some("The server refused the boss status query");
        return false;
    }
    let Some(snapshot) = boss_encounter::decode(&result.payload) else {
        panel.feedback = Some("The server returned invalid boss status");
        return false;
    };
    panel.snapshot = Some(snapshot);
    panel.feedback = None;
    true
}
