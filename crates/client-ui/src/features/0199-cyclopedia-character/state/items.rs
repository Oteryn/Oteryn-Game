use super::{CyclopediaCharacterPanel, Intent};
use oteryn_client_runtime::{
    cyclopedia_character::items::{self as wire, Location, Page, Row},
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemSummary {
    pub open: bool,
    pub page: Option<Page>,
    pub location: Option<Location>,
    pub search: String,
    pub pending_offset: Option<u16>,
}
impl ItemSummary {
    pub fn rows(&self) -> Vec<&Row> {
        let search = self.search.trim().to_lowercase();
        self.page.as_ref().map_or_else(Vec::new, |p| {
            p.rows
                .iter()
                .filter(|r| {
                    self.location.is_none_or(|l| r.quantities[l as usize] > 0)
                        && r.definition.to_lowercase().contains(&search)
                })
                .collect()
        })
    }
}
pub(super) fn request(p: &mut CyclopediaCharacterPanel, intent: Intent) -> Option<FeatureCommand> {
    if p.target.is_some() {
        p.feedback = Some("The item summary is available for your own character");
        return None;
    }
    let offset = match intent {
        Intent::ItemsNext => {
            let page = p.items.page.as_ref()?;
            let next = page.offset.checked_add(wire::PAGE_SIZE as u16)?;
            if next >= page.total {
                return None;
            }
            next
        }
        Intent::ItemsPrevious => {
            let offset = p.items.page.as_ref()?.offset;
            if offset == 0 {
                return None;
            }
            offset.saturating_sub(wire::PAGE_SIZE as u16)
        }
        _ => 0,
    };
    p.pending = true;
    p.command_kind = 388;
    p.feedback = None;
    p.items.pending_offset = Some(offset);
    Some(FeatureCommand {
        capability: 199,
        kind: 388,
        payload: wire::request(offset),
    })
}
pub(super) fn apply(p: &mut CyclopediaCharacterPanel, result: &GatedCommandResult) -> bool {
    let wanted = p.items.pending_offset.take();
    if !p.pending {
        return false;
    }
    p.pending = false;
    let page = matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    )
    .then(|| Page::decode(&result.payload).ok())
    .flatten()
    .filter(|page| Some(page.offset) == wanted);
    let Some(page) = page else {
        p.feedback = Some("The server refused or returned an invalid item summary");
        return false;
    };
    p.items.page = Some(page);
    p.feedback = None;
    true
}
