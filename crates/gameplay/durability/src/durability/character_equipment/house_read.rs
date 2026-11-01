//! Read-only equipment projection after explicit named House admission proof.
//! The sealed authority stays inside this transaction and cannot authorize a move.
use super::*;

/// Existing native equipment facts under the current House fence. This helper
/// never returns the sealed equipment authority and cannot authorize a writer.
/// The caller has already taken recovery and admission-relation locks.
pub(crate) async fn read_house_equipment_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    origin_channel: [u8; 16],
    content_digest: [u8; 32],
) -> Result<EquipmentSnapshot> {
    if content_digest == [0; 32] { return Err(rejected("equipment Content pin")); }
    let world = *fence.runtime_scope.world_id().as_bytes();
    sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:' || encode($1::bytea,'hex')::uuid::text || ':' || encode($2::bytea,'hex')::uuid::text,0))")
        .bind(world.as_slice()).bind(origin_channel.as_slice()).execute(&mut **tx).await?;
    if !super::super::house_hireling::fence::assert_house_inventory_fence(
        tx, node, fence, origin_channel,
    ).await? { return Err(rejected("current named House")); }
    let digest: Option<Vec<u8>> = sqlx::query_scalar("SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
        .bind(world.as_slice()).bind(origin_channel.as_slice()).fetch_optional(&mut **tx).await?;
    if digest.as_deref() != Some(content_digest.as_slice()) { return Err(rejected("current Content pin")); }
    let character = sqlx::query("SELECT character_revision::text,account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let authority = EquipmentAuthority {
        physical_transaction: sqlx::query_scalar("SELECT pg_current_xact_id()::text").fetch_one(&mut **tx).await?,
        fence: *fence, command: None, world, channel: origin_channel, content_digest,
        character_revision: number(character.try_get("character_revision")?)?,
        account: uuid(character.try_get("account_id")?)?,
    };
    read_equipment_in_transaction(tx, &authority).await
}

impl DurabilityRoot {
    pub async fn read_house_equipment(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        origin_channel: [u8; 16],
        content_digest: [u8; 32],
    ) -> Result<MovementEquipmentSnapshot> {
        let record = recovery.record_for(self).map_err(|_| rejected("equipment recovery root"))?;
        let node = node.clone();
        let fence = *fence;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            super::super::db::lock_admission_relations(&mut tx).await?;
            let equipment = read_house_equipment_in_transaction(&mut tx, &node, &fence,
                origin_channel, content_digest).await.map_err(|_| DurabilityError::Unavailable)?;
            super::super::db::commit_semantic_transaction(tx, deadline).await?;
            Ok(MovementEquipmentSnapshot { fence, equipment })
        })).await.map_err(EquipmentError::from)
    }
}
