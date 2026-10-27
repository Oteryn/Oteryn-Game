use crate::play::{Input, PlayView};
use crate::spell::SPELL_SLOTS;
use std::num::NonZeroU32;

impl PlayView {
    /// A spell hotkey: casts the 1-based spell-book entry. Entries past the bar are ignored and
    /// at most one press per bar slot waits for the next tick.
    pub fn cast(&mut self, spell: NonZeroU32) {
        if spell.get() <= u32::from(SPELL_SLOTS) {
            self.queue(Input::Cast(spell));
        }
    }
}

pub(in crate::play) fn send_cast(link: &crate::play::PlayLink, spell: NonZeroU32) {
    link.request_combat(crate::combat_input::CombatCommand::Cast(spell));
}
