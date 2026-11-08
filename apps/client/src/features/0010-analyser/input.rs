use crate::play::PlayLink;
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn analyser_select(&self, key: &str) -> bool {
        oteryn_client_ui::analyser::select_tool(
            &mut crate::play::mailbox::lock(&self.mailbox).analyser,
            key,
        )
    }
    pub fn analyser_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::analyser::open_key(events) {
            return false;
        }
        self.analyser_open();
        true
    }
    pub fn analyser_open(&self) {
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            crate::play::mailbox::lock(&self.mailbox).analyser.open = true;
        }
    }
}
