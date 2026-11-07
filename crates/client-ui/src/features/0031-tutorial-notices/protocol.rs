use super::NoticePanel;
use oteryn_client_runtime::tutorial_notices::Notice;
impl NoticePanel {
    pub fn apply(&mut self, notice: Notice) {
        match notice {
            Notice::Tutorial(text) => {
                if self.tutorial.len() == 32 {
                    self.tutorial.pop_front();
                }
                self.tutorial.push_back(text);
            }
            Notice::ServerSave { minutes } => self.save_minutes = Some(minutes),
        }
    }
}

pub fn apply(panel: &mut NoticePanel, notice: Notice) {
    panel.apply(notice);
}
