//! Producer-only catalogue capture; no housing acquisition or mutation authority.
use super::*;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{ChannelId, WorldId};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PublicHouseMetadata {
    pub house_key: String,
    pub name: String,
    pub kind: String,
    pub town: String,
    pub size_sqm: u32,
    pub beds: u16,
    pub rent_gold: u64,
    pub content_digest: String,
}

/// The caller supplies checked active bundle metadata. `None` withholds freshness;
/// `Some([])` is an explicit checked empty/no-bundle catalogue and withdraws exports.
pub struct PublicHouseCatalogue {
    pub world: WorldId,
    pub channel: ChannelId,
    pub generation: u64,
    pub reset_epoch: u64,
    pub map_revision: String,
    pub houses: Option<Vec<PublicHouseMetadata>>,
}

impl DurabilityRoot {
    pub async fn sync_public_house_catalogue(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        update: PublicHouseCatalogue,
    ) -> Result<()> {
        let _boot = self.boot_pass_deadline(super::RESYNC_DEADLINE);
        let recovery = authority.record_for(self)?;
        if update.generation == 0 || update.houses.as_ref().is_some_and(|rows| rows.len() > 1000) {
            return Err(CharacterAuthorityError::Rejected);
        }
        let payload = update.houses.map(serde_json::to_value).transpose()
            .map_err(|_| CharacterAuthorityError::Rejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let fact = node.fact();
            let applied: bool = sqlx::query_scalar(
                "SELECT game_public_house_catalogue_sync(encode($1,'hex')::uuid,encode($2,'hex')::uuid,\
                 $3::text::numeric,$4::text::numeric,$5,$6,encode($7,'hex')::uuid,$8::text::numeric,$9)")
                .bind(update.world.as_bytes().as_slice())
                .bind(update.channel.as_bytes().as_slice())
                .bind(update.generation.to_string())
                .bind(update.reset_epoch.to_string())
                .bind(update.map_revision)
                .bind(payload)
                .bind(fact.node_id().as_bytes().as_slice())
                .bind(fact.registration_revision().to_string())
                .bind(node.sql_possession_secret())
                .fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(if applied { Ok(()) } else { Err(CharacterAuthorityError::Rejected) })
        })).await?
    }
}
