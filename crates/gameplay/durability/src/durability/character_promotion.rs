//! Permanent promotion read by fenced owners; absence means never purchased.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{CharacterProgressionError, assert_item_only_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::npc_purchase::PurchaseReceipt;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::foundation::CommandRef;
use sqlx::PgConnection;

pub async fn read_durable_promotion(
    connection: &mut PgConnection,
    character: CharacterId,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_promotions WHERE character_id=encode($1,'hex')::uuid)")
        .bind(character.as_bytes().as_slice()).fetch_one(connection).await?)
}

impl DurabilityRoot {
    pub async fn read_promotion_for_session(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> Result<bool, CharacterProgressionError> {
        self.read_premium_purchase_state(authority, node, fence, None)
            .await
            .map(|(promoted, _)| promoted)
    }

    pub async fn reconcile_premium_purchase(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
    ) -> Result<Option<PurchaseReceipt>, CharacterProgressionError> {
        self.read_premium_purchase_state(authority, node, fence, Some(command))
            .await
            .map(|(_, receipt)| receipt)
    }

    async fn read_premium_purchase_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: Option<CommandRef>,
    ) -> Result<(bool, Option<PurchaseReceipt>), CharacterProgressionError> {
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
                    if let Err(error) = assert_item_only_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let promoted = read_durable_promotion(&mut tx, fence.character_id).await?;
                    let receipt = match command {
                        Some(command) => super::npc_purchase::load_by_command(
                            &mut tx,
                            fence.character_id,
                            command,
                        )
                        .await?
                        .map(|(_, receipt)| receipt),
                        None => None,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok((promoted, receipt)))
                })
            })
            .await?
    }
}
