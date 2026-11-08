use crate::play::PlayLink;
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn spell_catalog_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::spell_catalog::open_key(events) {
            return false;
        }
        self.spell_catalog_open();
        true
    }
    pub fn spell_catalog_assign(&self, slot: usize) -> bool {
        assign(&mut crate::play::mailbox::lock(&self.mailbox), slot)
    }
    pub fn spell_catalog_open(&self) {
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            crate::play::mailbox::lock(&self.mailbox).spell_catalog.open = true;
        }
    }
}

pub(super) fn assign(mailbox: &mut crate::play::mailbox::Mailbox, slot: usize) -> bool {
    let Some(spell) = mailbox
        .spell_catalog
        .selected_spell()
        .and_then(|entry| std::num::NonZeroU32::new(entry.cast_index))
    else {
        return false;
    };
    if !mailbox.spell_bar.bar.bind(slot, Some(spell)) {
        return false;
    }
    mailbox.spell_bar.feedback = format!("Selected spell assigned to slot {}", slot + 1);
    true
}
