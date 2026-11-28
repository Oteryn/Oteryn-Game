use super::*;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::house_ownership::{
    HOUSE_SETTLEMENT_PASS_LIMIT, HouseOwnershipError, HouseOwnershipOutcome,
    HouseSettlementPremium, HouseSettlementRequest,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::premium::IngestOutcome;
use crate::premium::client::{PremiumSnapshotClient, fresh_nonce};

/// Observe the same latch decision used for commands. Settlement rechecks that
/// decision in its own bounded transaction after any network requests.
struct Probe;
impl PreDeliveryBypass for Probe {
    type Output = ();
    fn apply<'a, 't>(
        self,
        _held: &'a EmptyLatchHeld,
        _tx: &'a mut PremiumGateTx<'t>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>
    where
        Self: 'a,
    {
        Box::pin(std::future::ready(()))
    }
}

impl SpellPremiumCoordinator {
    /// Settle through the configured activation and trusted clock. The worker
    /// pulls Platform directly for every bidder, including offline accounts.
    pub async fn settle_house_auction(
        &self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        client: Option<&PremiumSnapshotClient>,
        request: HouseSettlementRequest,
    ) -> Result<HouseOwnershipOutcome, HouseOwnershipError> {
        DurabilityRoot::with_background_holder(root.with_periodic_lock_budget(async {
            let premium = if request.step == 0 {
                match root.house_settlement_accounts(&request).await? {
                    Some(accounts) => self.house_election_premium(root, client, accounts).await?,
                    None => HouseSettlementPremium::Refund,
                }
            } else {
                HouseSettlementPremium::Refund
            };
            root.house_settlement(authority, node, request, premium)
                .await
        }))
        .await
    }

    async fn house_election_premium(
        &self,
        root: &DurabilityRoot,
        client: Option<&PremiumSnapshotClient>,
        accounts: Vec<[u8; 16]>,
    ) -> Result<HouseSettlementPremium, HouseOwnershipError> {
        // The probe moves activation forward at the switch-over through the
        // same durable latch owner as bids and other paid commands.
        let now = self.read_now();
        let activation = DurabilityRoot::with_semantic_pass_cutoff(
            tokio::time::Instant::now() + HOUSE_SETTLEMENT_PASS_LIMIT,
            self.settle(now, Probe),
        )
        .await?;
        match activation {
            Settled::Applied(()) => {
                let switch_over_us = self.delivery.activation.as_ref().map(|a| a.switch_over_us);
                if switch_over_us
                    .is_some_and(|cut| self.read_now().is_none_or(|t| t.upper_us() >= cut))
                {
                    return Err(HouseOwnershipError::NotPremium);
                }
                return Ok(HouseSettlementPremium::PreDelivery { switch_over_us });
            }
            Settled::Decided {
                delivered: true, ..
            } => {}
            Settled::Decided {
                delivered: false, ..
            } => return Err(HouseOwnershipError::NotPremium),
        }
        self.delivery
            .delivered_now(self.read_now())
            .ok_or(HouseOwnershipError::NotPremium)?;
        let client = client.ok_or(HouseOwnershipError::NotPremium)?;
        // Isolate the election from admission/release and live refresh races.
        // Network I/O happens outside the sole database holder.
        let consumer = PremiumConsumer::default();
        for account in &accounts {
            let nonce = fresh_nonce().ok_or(HouseOwnershipError::NotPremium)?;
            let body = client
                .pull(*account, &nonce)
                .await
                .map_err(|_| HouseOwnershipError::NotPremium)?;
            let accepted = DurabilityRoot::with_semantic_pass_cutoff(
                tokio::time::Instant::now() + HOUSE_SETTLEMENT_PASS_LIMIT,
                consumer.ingest(root, *account, &nonce, &body),
            )
            .await;
            if !matches!(accepted, IngestOutcome::Accepted | IngestOutcome::Replayed) {
                return Err(HouseOwnershipError::NotPremium);
            }
        }
        let now = self
            .delivery
            .delivered_now(self.read_now())
            .ok_or(HouseOwnershipError::NotPremium)?;
        let latch = self
            .delivery
            .held
            .get()
            .ok_or(HouseOwnershipError::NotPremium)?
            .clone();
        let mut valid_until_us = i64::MAX;
        let mut proofs = Vec::with_capacity(accounts.len());
        for account in accounts {
            let (revision, until, current) =
                if let Some(proof) = consumer.current_premium(account, Some(now)) {
                    (proof.authority_revision, proof.benefit_until_us, true)
                } else if let Some(proof) = consumer.current_premium_denial(account, now) {
                    (proof.authority_revision, proof.valid_until_us, false)
                } else {
                    return Err(HouseOwnershipError::NotPremium);
                };
            valid_until_us = valid_until_us.min(until);
            proofs.push((account, revision, current));
        }
        // Keep evidence valid throughout the bounded database pass, including
        // the trusted window's uncertainty.
        let budget_us = i64::try_from(HOUSE_SETTLEMENT_PASS_LIMIT.as_micros())
            .map_err(|_| HouseOwnershipError::InvalidInput)?;
        if now.upper_us().saturating_add(budget_us) >= valid_until_us {
            return Err(HouseOwnershipError::NotPremium);
        }
        Ok(HouseSettlementPremium::Delivered {
            latch,
            valid_until_us,
            accounts: proofs,
        })
    }
}
