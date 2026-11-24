use super::{HouseOwnershipError, Result};
use crate::durability::DurabilityRoot;
use crate::durability::db::{begin_semantic_transaction, commit_semantic_transaction};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{ChannelId, WorldId};

/// Register vacant properties from the bundle checked by the assigned channel.
pub struct HouseCatalogueActivation {
    pub world: WorldId,
    pub channel: ChannelId,
    pub generation: u64,
    pub sequence: u64,
    pub digest: [u8; 32],
    pub entries: Vec<serde_json::Value>,
}

impl DurabilityRoot {
    pub async fn activate_house_catalogue(
        &self,
        node: &NodeIncarnationProof,
        activation: HouseCatalogueActivation,
    ) -> Result<()> {
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let fact = node.fact();
                    let applied: bool = sqlx::query_scalar(
                        "SELECT game_house_catalogue_activate(encode($1,'hex')::uuid,\
                         encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5,$6,\
                         encode($7,'hex')::uuid,$8::text::numeric,$9)",
                    )
                    .bind(activation.world.as_bytes().as_slice())
                    .bind(activation.channel.as_bytes().as_slice())
                    .bind(activation.generation.to_string())
                    .bind(activation.sequence.to_string())
                    .bind(activation.digest.as_slice())
                    .bind(serde_json::Value::Array(activation.entries))
                    .bind(fact.node_id().as_bytes().as_slice())
                    .bind(fact.registration_revision().to_string())
                    .bind(node.sql_possession_secret())
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(if applied {
                        Ok(())
                    } else {
                        Err(HouseOwnershipError::AuthorityRejected)
                    })
                })
            })
            .await?
    }
}
