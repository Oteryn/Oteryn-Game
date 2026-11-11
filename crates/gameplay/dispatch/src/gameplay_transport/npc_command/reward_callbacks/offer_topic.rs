//! Source offers can explicitly restart a prompt from any conversation topic.
use super::Offer;

impl Offer {
    pub(super) fn matches_topic(&self, current: u16) -> bool {
        self.any_topic || self.topic == current
    }
}
