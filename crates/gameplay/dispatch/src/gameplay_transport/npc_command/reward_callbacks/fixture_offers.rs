//! Test fixtures enter through the production offer decoder and dialogue owner.
use super::*;

impl NpcConversations {
    #[cfg(any(
        feature = "pg-tests-g1",
        feature = "pg-tests-g2",
        feature = "pg-tests-g3",
        feature = "pg-tests-g4"
    ))]
    pub(in crate::gameplay_transport) fn fixture_reward_offers(
        &self,
        source: &str,
    ) -> Result<(), String> {
        let document: Offers = serde_json::from_str(source).map_err(|error| error.to_string())?;
        if document.schema != 1 || document.offers.len() > 512 {
            return Err("invalid offer fixture".into());
        }
        self.fixture_offer_rows
            .set(document.offers)
            .map_err(|_| "offer fixture already installed".into())
    }
}
