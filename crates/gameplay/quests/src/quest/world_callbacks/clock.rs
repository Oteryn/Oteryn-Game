//! Clock-only callbacks still require the personal fenced writer.
use super::Callback;

impl Callback {
    /// Item custody is fenced even when the source writes no personal progress.
    pub fn requires_durable_commit(&self) -> bool {
        self.has_personal_state() || super::super::dreamer_fire_bug::shared_input(self)
    }

    pub fn has_personal_state(&self) -> bool {
        !self.storage.is_empty() || !self.state_ops.is_empty()
    }
}
