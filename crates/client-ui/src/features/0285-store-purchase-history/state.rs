use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub offer_filter: String,
    pub from_ms: u64,
    pub until_ms: Option<u64>,
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub page: Option<StoreHistoryPage>,
    pub feedback: String,
}

use oteryn_client_runtime::discovery::store::StoreHistoryEntry;

pub fn set_filter(panel: &mut Panel, offer: &str, from: &str, until: &str) -> bool {
    if !panel.selected || panel.pending {
        return false;
    }
    let dates = (|| {
        let from = if from.trim().is_empty() {
            0
        } else {
            from.trim().parse::<u64>().ok()?
        };
        let until = if until.trim().is_empty() {
            None
        } else {
            Some(until.trim().parse::<u64>().ok()?)
        };
        until
            .is_none_or(|until| from <= until)
            .then_some((from, until))
    })();
    if offer.len() > 256 || offer.chars().any(char::is_control) || dates.is_none() {
        panel.feedback =
            "Enter a valid offer filter and inclusive Unix millisecond range (from <= through)"
                .into();
        return false;
    }
    if let Some((from, until)) = dates {
        panel.offer_filter = offer.trim().into();
        panel.from_ms = from;
        panel.until_ms = until;
    }
    panel.feedback =
        "Filters apply to the loaded purchase page; refresh or Next page for more records".into();
    true
}

pub(super) fn matches_filter(panel: &Panel, row: &StoreHistoryEntry) -> bool {
    row.offer_key
        .to_lowercase()
        .contains(&panel.offer_filter.to_lowercase())
        && row.purchased_at_unix_ms >= panel.from_ms
        && panel
            .until_ms
            .is_none_or(|until| row.purchased_at_unix_ms <= until)
}
