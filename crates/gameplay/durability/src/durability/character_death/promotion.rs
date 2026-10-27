//! Per-death Premium is evaluated in the fenced death transaction, never at login.
use super::*;
use std::{future::Future, pin::Pin};

pub trait DeathPremiumOwner: Sync {
    fn current<'a>(
        &'a self,
        tx: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<bool, DurabilityError>> + Send + 'a>>;
}

pub(super) async fn benefit(
    tx: &mut sqlx::PgConnection,
    character: CharacterId,
    premium: bool,
) -> std::result::Result<bool, DurabilityError> {
    let promoted =
        crate::durability::character_promotion::read_durable_promotion(tx, character).await?;
    Ok(crate::domain::premium::promotion_benefit_current(
        promoted,
        premium,
    ))
}

pub(super) async fn held_blessings(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
) -> Result<Vec<String>> {
    let recovery = authority
        .record_for(root)
        .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
    let node = node.clone();
    root.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
        let mut tx=begin_semantic_transaction(holder,deadline).await?;
        assert_recovery_fence(&mut tx,&recovery).await?;
        lock_admission_relations(&mut tx).await?;
        if let Err(error)=crate::durability::character_progression::assert_item_only_fence(&mut tx,&fence.item_fence(),&node).await? {return Ok(Err(error));}
        let held=sqlx::query_scalar("SELECT blessing_key COLLATE \"C\" FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid ORDER BY 1 LIMIT 33")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
        commit_semantic_transaction(tx,deadline).await?;
        Ok(Ok(held))
    })).await?
}
