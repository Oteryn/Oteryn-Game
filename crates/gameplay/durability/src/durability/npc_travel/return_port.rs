//! Fenced read of the last completed inbound passage, never movement/payment authority.
use super::*;
use crate::durability::character_progression::assert_gameplay_fence;
use oteryn_gameplay_content::content::npc_catalogue::travel::TravelReturnRequirement;

impl DurabilityRoot {
    /// Read the remembered inbound port under current character, session, node and scope fences.
    /// A travel commit must recheck this requirement inside its own transaction.
    pub async fn npc_travel_return_permitted(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        requirement: TravelReturnRequirement,
    ) -> Result<bool, CharacterProgressionError> {
        if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
            || !requirement.is_valid()
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let last = latest(&mut tx, *fence.character_id.as_bytes(), &requirement).await?;
                    let permitted = requirement.permits_latest_completed(last.as_deref());
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(permitted))
                })
            })
            .await?
    }
}

pub(super) async fn latest(
    connection: &mut sqlx::PgConnection,
    character: [u8; 16],
    requirement: &TravelReturnRequirement,
) -> Result<Option<String>, sqlx::Error> {
    if !requirement.is_valid() {
        return Ok(None);
    }
    sqlx::query_scalar(include_str!("return_port.sql"))
        .bind(character.as_slice())
        .bind(&requirement.inbound_routes)
        .fetch_one(connection)
        .await
}
