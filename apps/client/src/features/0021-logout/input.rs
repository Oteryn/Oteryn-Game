use crate::play::{Input, PlayView};

impl PlayView {
    /// The logout chord: at most one logout waits for the next tick.
    pub fn logout(&mut self) {
        self.queue(Input::Logout);
    }
}
