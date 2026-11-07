use super::protocol::feedback_text;
use oteryn_session::SpellCastDisposition;

/// The last cast's feedback line, shown until the next cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpellFeedback {
    last: Option<SpellCastDisposition>,
}

impl SpellFeedback {
    pub const fn record(&mut self, disposition: SpellCastDisposition) {
        self.last = Some(disposition);
    }

    #[must_use]
    pub const fn text(&self) -> Option<&'static str> {
        match self.last {
            Some(disposition) => Some(feedback_text(disposition)),
            None => None,
        }
    }
}

#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
