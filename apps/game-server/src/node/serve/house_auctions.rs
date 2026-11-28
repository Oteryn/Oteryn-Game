use super::*;
use crate::durability::house_ownership::{HouseAuctionCatalogue, HouseAuctionProperty};

pub(super) struct HouseAuctions<'a, 'f, 's> {
    pub(super) root: &'a DurabilityRoot,
    pub(super) authority: &'a ReconciledCharacterAuthority<'f, 's>,
    pub(super) node: &'a NodeIncarnationProof,
    pub(super) catalogue: HouseAuctionCatalogue<'a>,
}

type AuctionProperty = (String, &'static str, u64, Vec<[i32; 3]>);

impl HouseAuctions<'_, '_, '_> {
    pub(super) async fn run(
        self,
        bundle: &crate::map::boot::BundleWorld,
        stop: &CancellationToken,
    ) {
        DurabilityRoot::with_background_holder(self.run_loop(bundle, stop)).await;
    }

    async fn run_loop(self, bundle: &crate::map::boot::BundleWorld, stop: &CancellationToken) {
        let mut pending: Vec<_> = super::public_house_catalogue::auction_properties(bundle)
            .into_iter()
            .flatten()
            .collect();
        let mut cursor = None;
        loop {
            let Some(retry) = self.qualify_listings(pending, stop).await else {
                return;
            };
            pending = retry;

            let Some(result) = super::super::cancellation::first(
                self.root
                    .due_house_auctions(self.catalogue.scope.world_id(), cursor),
                stop.cancelled(),
            )
            .await
            else {
                return;
            };
            if let Ok(batch) = result {
                cursor = batch.last().map(|request| request.auction);
                for request in batch {
                    if super::super::cancellation::first(
                        self.root
                            .house_settlement(self.authority, self.node, request),
                        stop.cancelled(),
                    )
                    .await
                    .is_none()
                    {
                        return;
                    }
                }
            }
            if super::super::cancellation::first(
                tokio::time::sleep(std::time::Duration::from_secs(5)),
                stop.cancelled(),
            )
            .await
            .is_none()
            {
                return;
            }
        }
    }
    async fn qualify_listings(
        &self,
        pending: Vec<AuctionProperty>,
        stop: &CancellationToken,
    ) -> Option<Vec<AuctionProperty>> {
        let mut retry = Vec::new();
        for (key, kind, rent, tiles) in pending {
            let property = HouseAuctionProperty {
                key: &key,
                kind,
                rent,
                tiles: &tiles,
            };
            match super::super::cancellation::first(
                self.root.qualify_house_auction(
                    self.authority,
                    self.node,
                    &self.catalogue,
                    property,
                ),
                stop.cancelled(),
            )
            .await
            {
                None => return None,
                Some(Ok(true)) => continue,
                Some(Ok(false) | Err(_)) => retry.push((key, kind, rent, tiles)),
            }
        }
        Some(retry)
    }
}

#[cfg(test)]
#[path = "house_auctions_tests.rs"]
mod tests;
