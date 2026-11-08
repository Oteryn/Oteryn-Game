//! Clock-only callbacks still require the personal fenced writer.
use super::Callback;

impl Callback {
    pub fn has_personal_state(&self) -> bool {
        !self.storage.is_empty() || !self.state_ops.is_empty()
    }
}
