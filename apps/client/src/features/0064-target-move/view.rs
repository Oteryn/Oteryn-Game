use crate::play::{PlayLink, mailbox::lock};

impl PlayLink {
    pub(in crate::play) fn target_walk_feedback(&self) -> Option<&'static str> {
        oteryn_client_ui::target_move::view::feedback(&lock(&self.mailbox).target_walk)
    }

    pub fn target_walk_state(&self) -> oteryn_client_ui::target_move::WalkState {
        lock(&self.mailbox).target_walk.clone()
    }
}

impl crate::play::PlayView {
    pub fn target_walk_feedback(&self) -> Option<&'static str> {
        self.target_walk_feedback
    }
}
