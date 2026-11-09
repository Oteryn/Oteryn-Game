//! Imbuement shrine panel state and command construction.
#[path = "../0447-imbuement-details/protocol.rs"]
pub mod details;
#[path = "state/equipment_tracker.rs"]
pub mod equipment_tracker;
mod input;
mod view;
pub use input::{open_key, tracker_key};
pub use oteryn_client_runtime::imbue::{ImbueCommand, ImbueResponse, ItemHandle, imbuement};
pub use view::{panel_text, selected_detail_text, slot_label, tracker_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Apply,
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImbuePanel {
    pub details_selected: bool,
    pub details_pending: Option<ItemHandle>,
    pub details_attempted: bool,
    pub details: Option<oteryn_client_runtime::imbuement_details::Reply>,
    pub details_feedback: Option<&'static str>,
    pub selected: bool,
    pub open: bool,
    pub tracker_open: bool,
    pub equipment: equipment_tracker::EquipmentTracker,
    pub tracker_minutes: String,
    pub pending: bool,
    pub refresh_required: bool,
    pub item: Option<ItemHandle>,
    pub slot: u8,
    pub offer: usize,
    pub window: Option<imbuement::ImbuementWindow>,
    pub feedback: Option<&'static str>,
}

impl ImbuePanel {
    pub fn invalidate_item_scope(&mut self) {
        *self = Self {
            selected: self.selected,
            details_selected: self.details_selected,
            open: self.open,
            tracker_open: self.tracker_open,
            tracker_minutes: self.tracker_minutes.clone(),
            feedback: Some("Select an item in the current activity and refresh it"),
            ..Self::default()
        };
    }

    pub fn select_item(&mut self, item: ItemHandle) {
        if self.pending || self.item == Some(item) {
            return;
        }
        self.item = Some(item);
        self.window = None;
        self.details = None;
        self.details_pending = None;
        self.details_attempted = false;
        self.details_feedback = None;
        self.refresh_required = false;
        self.slot = 0;
        self.offer = 0;
        self.feedback = Some("Refresh the selected item at the shrine");
    }
    pub fn select_slot(&mut self, slot: u8) -> bool {
        if !self.selected
            || self.pending
            || self
                .window
                .as_ref()
                .is_none_or(|window| slot >= window.slot_count)
        {
            return false;
        }
        self.slot = slot;
        true
    }
    pub fn select_offer(&mut self, offer: usize) -> bool {
        if !self.selected
            || self.pending
            || self
                .window
                .as_ref()
                .is_none_or(|window| offer >= window.offers.len())
        {
            return false;
        }
        self.offer = offer;
        true
    }
    pub fn can_apply(&self) -> bool {
        self.selected
            && self.item.is_some()
            && !self.pending
            && self.window.as_ref().is_some_and(|window| {
                self.slot < window.slot_count
                    && self.offer < window.offers.len()
                    && !window
                        .applied
                        .iter()
                        .any(|applied| applied.slot == self.slot)
                    && (!self.details_selected
                        || self.details.as_ref().is_some_and(|details| {
                            let offer = &window.offers[self.offer];
                            Some(details.item) == self.item
                                && details.quotes.iter().any(|quote| {
                                    quote.family == offer.family
                                        && quote.tier == offer.tier
                                        && quote.recipe_available
                                })
                        }))
            })
    }
    pub fn can_clear(&self) -> bool {
        self.selected
            && self.item.is_some()
            && !self.pending
            && self.window.as_ref().is_some_and(|window| {
                self.slot < window.slot_count
                    && window
                        .applied
                        .iter()
                        .any(|applied| applied.slot == self.slot)
            })
    }
}

pub fn request(panel: &mut ImbuePanel, intent: Intent) -> Option<ImbueCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let item = match panel.item {
        Some(item) => item,
        None => {
            panel.feedback = Some("Select an item to imbue");
            return None;
        }
    };
    let action = match intent {
        Intent::Refresh => {
            panel.refresh_required = false;
            imbuement::ImbuementAction::Query
        }
        Intent::Apply => {
            if !panel.can_apply() {
                panel.feedback = Some(
                    "Select an empty slot and an available offer; refresh missing material details",
                );
                return None;
            }
            let window = panel.window.as_ref()?;
            (panel.slot < window.slot_count).then_some(())?;
            let index = u8::try_from(panel.offer).ok()?;
            window.offers.get(panel.offer)?;
            imbuement::ImbuementAction::Apply {
                slot: panel.slot,
                imbuement: index,
            }
        }
        Intent::Clear => {
            if !panel.can_clear() {
                panel.feedback = Some("Select an occupied slot to clear");
                return None;
            }
            (panel.slot < panel.window.as_ref()?.slot_count).then_some(())?;
            imbuement::ImbuementAction::Clear { slot: panel.slot }
        }
    };
    panel.pending = true;
    panel.feedback = None;
    Some(ImbueCommand(imbuement::ImbuementRequest { item, action }))
}

pub fn apply_result(panel: &mut ImbuePanel, response: &ImbueResponse) {
    panel.pending = false;
    let Some(reply) = &response.reply else {
        panel.feedback = Some("The server refused the imbuement request");
        return;
    };
    panel.feedback = match reply.result {
        imbuement::ImbuementResult::Ok => None,
        imbuement::ImbuementResult::RequirementNotMet => Some("Requirements are not met"),
        imbuement::ImbuementResult::MaterialsMissing => Some("Required materials are missing"),
        imbuement::ImbuementResult::InsufficientFunds => Some("Insufficient funds"),
        imbuement::ImbuementResult::SlotOccupied => Some("That slot is occupied"),
        imbuement::ImbuementResult::NotAllowed => Some("This imbuement is not allowed"),
        imbuement::ImbuementResult::RevisionChanged => Some("The item changed; refresh it"),
    };
    if let Some(window) = &reply.window {
        panel.window = Some(window.clone());
        panel.details = None;
        panel.details_pending = None;
        panel.details_attempted = false;
        panel.details_feedback = None;
        panel.slot = panel.slot.min(window.slot_count.saturating_sub(1));
        panel.offer = panel.offer.min(window.offers.len().saturating_sub(1));
    } else if matches!(
        reply.result,
        imbuement::ImbuementResult::Ok | imbuement::ImbuementResult::RevisionChanged
    ) {
        // Mutation replies carry no new slot snapshot. Never present the old quote as current.
        panel.window = None;
        panel.details = None;
        panel.details_pending = None;
        panel.details_attempted = false;
        panel.refresh_required = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU64;

    #[test]
    fn actions_are_built_from_loaded_server_state() {
        let mut panel = ImbuePanel {
            selected: true,
            item: NonZeroU64::new(7),
            slot: 1,
            window: Some(imbuement::ImbuementWindow {
                slot_count: 2,
                applied: vec![],
                offers: vec![imbuement::ImbuementOffer {
                    family: 4,
                    tier: 2,
                    gold: 90,
                    requires_premium: false,
                }],
                clear_gold: 12,
            }),
            ..ImbuePanel::default()
        };
        assert_eq!(
            request(&mut panel, Intent::Apply).map(|command| command.0.action),
            Some(imbuement::ImbuementAction::Apply {
                slot: 1,
                imbuement: 0
            })
        );
    }

    #[test]
    fn refusal_is_visible_and_success_replaces_the_window() {
        let mut panel = ImbuePanel {
            pending: true,
            ..ImbuePanel::default()
        };
        apply_result(
            &mut panel,
            &ImbueResponse {
                status: oteryn_client_runtime::imbue::CommandStatus::Accepted,
                reply: Some(imbuement::ImbuementReply {
                    result: imbuement::ImbuementResult::MaterialsMissing,
                    window: None,
                }),
            },
        );
        assert!(panel_text(&panel).contains("materials are missing"));
    }
}

#[cfg(test)]
#[path = "tests/tracker.rs"]
mod tracker_cases;
