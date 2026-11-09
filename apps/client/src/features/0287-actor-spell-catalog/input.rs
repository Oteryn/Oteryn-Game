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
    pub fn spell_catalog_open(&self) {
        if self.spellbook_open() {
            return;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            crate::play::mailbox::lock(&self.mailbox).spell_catalog.open = true;
        }
    }
}
