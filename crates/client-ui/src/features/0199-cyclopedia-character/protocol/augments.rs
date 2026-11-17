//! A single atomic observation page, with source filtering and ordering on the server.
use super::CyclopediaCharacterPanel;
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{
        CharacterTab,
        offence::{Page, Query},
    },
    raw_feature::{FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AugmentPanel {
    pub open: bool,
    pub query: Query,
    pub requested: Option<Query>,
    pub page: Option<Page>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AugmentIntent {
    Refresh,
    Next,
    Previous,
    Sources(u8),
    Sort,
}
pub fn request(p: &mut CyclopediaCharacterPanel, intent: AugmentIntent) -> Option<FeatureCommand> {
    if !p.selected || p.pending || p.blessings.pending {
        p.feedback = Some("Spell modifiers are unavailable or waiting for the server");
        return None;
    }
    let mut query = p.augments.query;
    query.target = p.target;
    match intent {
        AugmentIntent::Refresh => query.page = 0,
        AugmentIntent::Next => {
            let page = p.augments.page.as_ref().filter(|page| page.has_more())?;
            query.page = page.query.page.checked_add(1)?;
        }
        AugmentIntent::Previous => {
            query.page = p.augments.page.as_ref()?.query.page.saturating_sub(1)
        }
        AugmentIntent::Sources(sources) => {
            query.sources = sources;
            query.page = 0;
        }
        AugmentIntent::Sort => {
            query.by_source = !query.by_source;
            query.page = 0;
        }
    }
    let Ok(payload) = query.encode() else {
        p.feedback = Some("Choose a supported modifier source filter");
        return None;
    };
    p.profile_pending = false;
    p.auxiliary_pending = false;
    p.misc_resources_due = false;
    p.requested_tab = None;
    p.tab = CharacterTab::Offence;
    p.augments.open = true;
    p.augments.query = query;
    p.augments.requested = Some(query);
    p.augments.page = None;
    p.pending = true;
    p.command_kind = 388;
    p.feedback = None;
    Some(FeatureCommand {
        capability: 199,
        kind: 388,
        payload,
    })
}
pub(super) fn apply(p: &mut CyclopediaCharacterPanel, result: &GatedCommandResult) -> bool {
    let Some(query) = p.augments.requested.take() else {
        return false;
    };
    p.pending = false;
    p.augments.page = None;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.feedback = Some("The server refused the spell modifier query");
        return false;
    }
    let Ok(page) = Page::decode(&result.payload) else {
        p.feedback = Some("The server returned invalid spell modifiers");
        return false;
    };
    if query.target.is_some_and(|target| target != page.character)
        || (page.query.page, page.query.sources, page.query.by_source)
            != (query.page, query.sources, query.by_source)
    {
        p.feedback = Some("The server returned a different spell modifier view");
        return false;
    }
    p.combat = Some(page.stats.clone());
    p.augments.page = Some(page);
    p.feedback = None;
    true
}
#[cfg(test)]
#[path = "../tests/augments.rs"]
mod tests;
