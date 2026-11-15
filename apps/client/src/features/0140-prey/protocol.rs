use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::prey::PreyResponse;

impl Outbox {
    pub(in crate::play) fn prey_initialize(&self, selected: bool) {
        lock(&self.0).prey.selected = selected;
    }

    pub(in crate::play) fn prey_result(&self, response: &PreyResponse) {
        oteryn_client_ui::prey::apply_result(&mut lock(&self.0).prey, response);
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl Outbox {
    pub(crate) fn bevy_prey_initialize(&self, selected: bool) {
        self.prey_initialize(selected);
    }
    pub(crate) fn bevy_prey_result(&self, response: &PreyResponse) {
        self.prey_result(response);
    }
}

/// Native controls ask before any action that can spend gold or wildcard cards.
#[cfg_attr(not(windows), allow(dead_code, reason = "Native confirmation is exercised by play tests"))]
pub(in crate::play) fn confirm_action(
    intent: oteryn_client_ui::prey::Intent,
    confirm: impl FnOnce(&str) -> bool,
) -> Option<oteryn_client_ui::prey::Intent> {
    use oteryn_client_ui::prey::Intent;
    let cost = match &intent {
        Intent::ListReroll { .. } => Some("Reroll this list? The cost is 200 gold per character level when a free reroll is unavailable."),
        Intent::BonusReroll { .. } => Some("Reroll this bonus for 1 Prey wildcard card?"),
        Intent::SelectAny { .. } => Some("Select this creature for 5 Prey wildcard cards?"),
        _ => None,
    };
    if cost.is_some_and(|cost| !confirm(cost)) {
        None
    } else {
        Some(intent)
    }
}
