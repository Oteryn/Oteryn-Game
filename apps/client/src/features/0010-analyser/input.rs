use crate::play::PlayLink;
use oteryn_input_actions::NormalizedInputEvent;
#[cfg(any(windows, test))]
pub(in crate::play) fn select_tool(panel: &mut oteryn_client_ui::analyser::Panel, index: usize) {
    if let Some(tool) = oteryn_client_ui::analyser::Tool::ALL.get(index) {
        panel.tool = *tool;
    } else if index == 7 {
        oteryn_client_ui::analyser::protocol::reset(panel);
    }
}

impl PlayLink {
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
