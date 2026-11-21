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
    pub names: std::collections::BTreeMap<String, String>,
    pub pending_offset: Option<u16>,
}
impl ItemSummary {
    pub fn display_name<'a>(&'a self, row: &'a Row) -> &'a str {
        self.names
            .get(&row.definition)
            .map_or(row.definition.as_str(), String::as_str)
    }

    pub fn rows(&self) -> Vec<&Row> {
        let search = self.search.trim().to_lowercase();
        self.page.as_ref().map_or_else(Vec::new, |p| {
            p.rows
                .iter()
                .filter(|r| {
                    self.location.is_none_or(|l| r.quantities[l as usize] > 0)
                        && (r.definition.to_lowercase().contains(&search)
                            || self.display_name(r).to_lowercase().contains(&search))
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
    let payload = if p.holdings_selected {
        wire::filtered::Query {
            offset,
            location: p.items.location,
            search: p.items.search.trim().into(),
        }
        .encode()
        .ok()?
    } else {
        wire::request(offset)
    };
    p.pending = true;
    p.command_kind = 388;
    p.feedback = None;
    p.items.pending_offset = Some(offset);
    Some(FeatureCommand {
        capability: 199,
        kind: 388,
        payload,
    })
}
pub(super) fn apply(p: &mut CyclopediaCharacterPanel, result: &GatedCommandResult) -> bool {
    let wanted = p.items.pending_offset.take();
    if !p.pending {
        return false;
    }
    p.pending = false;
    let decoded = if p.holdings_selected {
        wire::filtered::Page::decode(&result.payload)
            .map(|page| (page.holdings, page.names))
            .ok()
    } else {
        Page::decode(&result.payload)
            .map(|page| (page, Vec::new()))
            .ok()
    };
    let page = matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    )
    .then_some(decoded)
    .flatten()
    .filter(|(page, _)| Some(page.offset) == wanted);
    let Some((page, names)) = page else {
        p.feedback = Some("The server refused or returned an invalid item summary");
        return false;
    };
    p.items.names = page
        .rows
        .iter()
        .zip(names)
        .map(|(row, name)| (row.definition.clone(), name))
        .collect();
    p.items.page = Some(page);
    p.feedback = None;
    true
}

pub fn filter(
    p: &mut CyclopediaCharacterPanel,
    search: &str,
    location: Option<Location>,
) -> Option<FeatureCommand> {
    if !p.selected
        || !p.holdings_selected
        || p.pending
        || search.len() > wire::filtered::MAX_SEARCH_BYTES
    {
        return None;
    }
    wire::filtered::Query {
        offset: 0,
        location,
        search: search.into(),
    }
    .encode()
    .ok()?;
    p.items.search = search.into();
    p.items.location = location;
    request(p, Intent::Items)
}
