use super::*;
use crate::durability::admission_authority_guards::uuid_text;
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::{
    ReconciledCharacterAuthority, assert_recovery_fence,
};
use crate::durability::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::RuntimeScopeRefV1;

pub struct HouseAuctionCatalogue<'a> {
    pub scope: RuntimeScopeRefV1,
    pub generation: u64,
    pub reset_epoch: u64,
    pub map_revision: &'a str,
    pub content_digest: [u8; 32],
}

pub struct HouseAuctionProperty<'a> {
    pub key: &'a str,
    pub kind: &'static str,
    pub rent: u64,
    pub tiles: &'a [[i32; 3]],
}

impl DurabilityRoot {
    /// Register one auction listing from a checked active map. The first bid
    /// materializes its property and verifies any existing custody addresses.
    pub async fn qualify_house_auction(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        catalogue: &HouseAuctionCatalogue<'_>,
        property: HouseAuctionProperty<'_>,
    ) -> Result<bool> {
        let RuntimeScopeRefV1::Channel { world_id, channel_id } = catalogue.scope else {
            return Err(HouseOwnershipError::AuthorityRejected);
        };
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseOwnershipError::AuthorityRejected)?;
        let node = node.clone();
        let payload = serde_json::json!({
            "world": uuid_text(world_id.as_bytes()),
            "channel": uuid_text(channel_id.as_bytes()),
            "generation": catalogue.generation.to_string(),
            "epoch": catalogue.reset_epoch.to_string(),
            "map": catalogue.map_revision,
            "digest": hex(&catalogue.content_digest),
            "house": property.key,
            "kind": property.kind,
            "rent": property.rent,
            "tiles": property.tiles,
        });
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                let fact = node.fact();
                let qualified = sqlx::query_scalar(
                    "SELECT game_house_qualify_auction($1,encode($2,'hex')::uuid,$3::text::numeric,$4)",
                )
                .bind(payload)
                .bind(fact.node_id().as_bytes().as_slice())
                .bind(fact.registration_revision().to_string())
                .bind(node.sql_possession_secret())
                .fetch_one(&mut *tx)
                .await?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(qualified)
            }))
            .await
            .map_err(Into::into)
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
