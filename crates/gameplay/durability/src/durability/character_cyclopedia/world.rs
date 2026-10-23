//! Session-independent epoch preparation, under the existing Channel timer fence.
use super::*;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::RuntimeScopeRefV1;
#[derive(Debug, Clone, Copy)]
pub struct CyclopediaWorldBinding {
    pub scope: RuntimeScopeRefV1,
    pub generation: u64,
    pub bundle: [u8; 32],
    pub epoch: u64,
}
impl DurabilityRoot {
    pub async fn prepare_cyclopedia_epoch(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        binding: CyclopediaWorldBinding,
    ) -> Result<bool, DurabilityError> {
        let mut context = (authority, node, permit);
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move |holder,deadline,context|Box::pin(async move {
            let (authority,node,permit)=context;
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(&mut tx,permit,self,authority,node,binding.scope,binding.generation)
                .await.map_err(|_|DurabilityError::Unavailable)?;
            let active=sqlx::query("SELECT a.reset_epoch::text,e.target_digest FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid FOR SHARE OF a")
                .bind(binding.scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            if !active.as_ref().is_some_and(|r|numeric_u64(r,"reset_epoch").ok()==Some(binding.epoch) && r.try_get::<Vec<u8>,_>("target_digest").ok().as_deref()==Some(binding.bundle.as_slice())) {return Err(DurabilityError::Unavailable);}
            let retiring:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid AND phase='RETIRING')")
                .bind(binding.scope.world_id().as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if retiring {return Err(DurabilityError::Unavailable);}
            let (_,improved)=storage::world(&mut tx,binding.scope.world_id(),binding.bundle,binding.epoch).await?;
            tx.commit(deadline).await?;
            Ok(improved)
        })).await
    }
}
