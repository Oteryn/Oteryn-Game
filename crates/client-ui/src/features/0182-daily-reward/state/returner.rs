//! Returner flow state; ordinary daily rewards never manufacture a returner entitlement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnerOffer {
    pub id: [u8; 16],
    pub expires_epoch: u64,
    pub description: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReturnerRewardPanel {
    pub offer: Option<ReturnerOffer>,
    pub pending: bool,
    pending_offer: Option<[u8; 16]>,
    claimed_offer: Option<[u8; 16]>,
}
impl ReturnerRewardPanel {
    pub fn notification(&self) -> String {
        self.offer.as_ref().map_or_else(
            || "No returner reward announced".into(),
            |offer| {
                format!(
                    "Welcome back!\n{}\n{}",
                    offer.description,
                    if self.claimed_offer == Some(offer.id) {
                        "Claimed"
                    } else if self.pending {
                        "Waiting for the server"
                    } else {
                        "Returner reward available"
                    }
                )
            },
        )
    }
    /// Returns an entitlement identity for a server claim, never a local grant.
    pub fn claim(&mut self, now_epoch: u64) -> Option<[u8; 16]> {
        let offer = self.offer.as_ref()?;
        if self.pending
            || self.claimed_offer == Some(offer.id)
            || offer.expires_epoch <= now_epoch
            || offer.id == [0; 16]
        {
            return None;
        }
        self.pending = true;
        self.pending_offer = Some(offer.id);
        Some(offer.id)
    }
    pub fn finish(&mut self, id: [u8; 16], accepted: bool) -> bool {
        if !self.pending || self.pending_offer != Some(id) {
            return false;
        }
        self.pending = false;
        self.pending_offer = None;
        if accepted {
            self.claimed_offer = Some(id);
        }
        true
    }
}
