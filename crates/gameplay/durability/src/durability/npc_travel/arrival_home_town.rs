//! Pending-arrival metadata is not authority to change citizenship or place an actor.
use super::*;

impl DurabilityRoot {
    /// Resolve a current pending occurrence's required town under the existing read fence.
    /// The caller must independently admit that town from its current World and consume
    /// through the atomic hometown owner. Completed/death-cancelled occurrences return None.
    pub async fn npc_pending_arrival_home_town(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: TravelOccurrence,
    ) -> Result<Option<u32>, TravelStoreError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| TravelStoreError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = async {
                        let pending = read_fenced(&mut tx, &recovery, &node, fence).await?;
                        if !pending.is_some_and(|row| row.occurrence == occurrence) {
                            return Ok(None);
                        }
                        target(&mut tx, fence.character_id, occurrence).await
                    }
                    .await;
                    if result.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await?
    }
}

pub(super) async fn target(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    occurrence: TravelOccurrence,
) -> Result<Option<u32>, TravelStoreError> {
    let targuna: Option<bool> = sqlx::query_scalar(
        "SELECT route='targuna' FROM game_newhaven_travel_receipts WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid"
    ).bind(character.as_bytes().as_slice()).bind(occurrence.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    Ok(targuna.unwrap_or(false).then_some(34))
}
