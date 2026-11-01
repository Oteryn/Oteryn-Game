use super::prey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickConfirmation {
    pub slot: u8,
    pub creature: String,
    pub price_gold: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreyPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: Option<prey::Request>,
    pub confirmed: Option<prey::Reply>,
    pub candidate: [Option<u8>; 3],
    pub offered_slot: Option<u8>,
    pub offered_today: Vec<prey::OfferedCreature>,
    pub offered_next: String,
    pub confirmation: Option<PickConfirmation>,
    pub feedback: Option<&'static str>,
}

impl PreyPanel {
    pub fn slot(&self, slot: u8) -> Option<&prey::Slot> {
        self.confirmed
            .as_ref()?
            .slots
            .iter()
            .find(|state| state.slot == slot)
    }

    pub fn unlocked(&self, slot: u8) -> bool {
        self.selected && self.slot(slot).is_some_and(|state| state.state != 0)
    }

    pub fn choose_candidate(&mut self, slot: u8, index: u8) -> bool {
        if self.pending.is_some()
            || !self.unlocked(slot)
            || self
                .slot(slot)
                .is_none_or(|state| index == 0 || usize::from(index) > state.creatures.len())
        {
            return false;
        }
        self.candidate[usize::from(slot - 1)] = Some(index);
        true
    }

    /// A quoted entry is only a proposal. Confirming sends the exact quoted amount.
    pub fn confirm_offer(&mut self, row: usize) -> Option<PickConfirmation> {
        let slot = self.offered_slot?;
        if self.pending.is_some() || !self.unlocked(slot) {
            return None;
        }
        let offered = self.offered_today.get(row)?;
        if offered.occupied_slot != 0 && offered.occupied_slot != slot {
            return None;
        }
        let confirmation = PickConfirmation {
            slot,
            creature: offered.creature.clone(),
            price_gold: offered.price_gold,
        };
        self.confirmation = Some(confirmation.clone());
        Some(confirmation)
    }
}
