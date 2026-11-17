use super::CyclopediaCharacterPanel;
use crate::account_achievements::{self as browser, Intent};
use oteryn_client_runtime::{
    cyclopedia_character::achievements::{AchievementPage, AchievementRequest},
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
fn command(p: &mut CyclopediaCharacterPanel, page: u32) -> Option<FeatureCommand> {
    let Ok(page) = u8::try_from(page) else {
        p.achievements.pending = None;
        p.achievements.catalog.collecting = false;
        p.achievements.error = Some(tr!("The achievement list exceeded its page limit"));
        return None;
    };
    p.pending = true;
    p.achievement_pending = true;
    p.command_kind = 388;
    p.requested_tab = None;
    p.feedback = None;
    Some(FeatureCommand {
        capability: 199,
        kind: 388,
        payload: AchievementRequest {
            page,
            target: p.target,
        }
        .encode(),
    })
}
pub fn request(p: &mut CyclopediaCharacterPanel, intent: Intent) -> Option<FeatureCommand> {
    if !p.selected || p.pending {
        return None;
    }
    p.achievements.selected = p.selected;
    p.achievements.catalog.enabled = true;
    let page = browser::request(&mut p.achievements, intent)?;
    if matches!(intent, Intent::Refresh) {
        p.achievement_profile = None;
    }
    command(p, page)
}
pub fn next_request(p: &mut CyclopediaCharacterPanel) -> Option<FeatureCommand> {
    if !p.selected || p.pending {
        return None;
    }
    let page = browser::catalog_next_request(&mut p.achievements)?;
    command(p, page)
}
pub(super) fn apply(p: &mut CyclopediaCharacterPanel, r: &GatedCommandResult) -> bool {
    if !p.pending {
        return false;
    }
    p.pending = false;
    p.achievement_pending = false;
    let page = matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    )
    .then(|| AchievementPage::decode(&r.payload).ok())
    .flatten()
    .filter(|page| p.target.is_none_or(|target| page.character == target))
    .filter(|page| {
        p.achievement_profile
            .as_ref()
            .is_none_or(|(id, name)| *id == page.character && *name == page.name)
    });
    let Some(page) = page else {
        browser::apply_result(
            &mut p.achievements,
            &oteryn_client_runtime::account_achievements::Response::Refused,
        );
        p.feedback = Some(tr!(
            "The server refused or returned an invalid character achievement page"
        ));
        return false;
    };
    p.achievement_profile = Some((page.character, page.name));
    browser::apply_result(
        &mut p.achievements,
        &oteryn_client_runtime::account_achievements::Response::Page(page.page),
    );
    p.feedback = p.achievements.error;
    p.feedback.is_none()
}
