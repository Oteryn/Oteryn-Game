//! Imbuement shrine panel state and command construction.
mod input;
pub use input::open_key;
pub use oteryn_client_runtime::imbue::{ImbueCommand, ImbueResponse, ItemHandle, imbuement};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Apply,
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImbuePanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub item: Option<ItemHandle>,
    pub slot: u8,
    pub offer: usize,
    pub window: Option<imbuement::ImbuementWindow>,
    pub feedback: Option<&'static str>,
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
        Intent::Refresh => imbuement::ImbuementAction::Query,
        Intent::Apply => {
            let index = u8::try_from(panel.offer).ok()?;
            panel.window.as_ref()?.offers.get(panel.offer)?;
            imbuement::ImbuementAction::Apply {
                slot: panel.slot,
                imbuement: index,
            }
        }
        Intent::Clear => {
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
        panel.slot = panel.slot.min(window.slot_count.saturating_sub(1));
        panel.offer = panel.offer.min(window.offers.len().saturating_sub(1));
    }
}

#[must_use]
pub fn panel_text(panel: &ImbuePanel) -> String {
    let mut text = if let Some(window) = &panel.window {
        format!(
            "{} slots; clear cost: {} gold\n",
            window.slot_count, window.clear_gold
        )
    } else {
        "No imbuement state loaded\n".to_owned()
    };
    if let Some(window) = &panel.window {
        for applied in &window.applied {
            text.push_str(&format!(
                "Slot {}: family {}, tier {}, {} ms remaining\n",
                applied.slot, applied.family, applied.tier, applied.remaining_ms
            ));
        }
        for (index, offer) in window.offers.iter().enumerate() {
            text.push_str(&format!(
                "Offer {index}: family {}, tier {}, {} gold{}\n",
                offer.family,
                offer.tier,
                offer.gold,
                if offer.requires_premium {
                    " (premium)"
                } else {
                    ""
                }
            ));
        }
    }
    if panel.pending {
        text.push_str("Request pending\n");
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
    }
    text
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
