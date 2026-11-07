//! The existing native fixed-map CAS runs while the real named-House read locks remain held.
use super::*;
impl DurabilityRoot {
    /// The callback changes only the existing map owner. Its published result
    /// remains in `owner` even if cleanup of this read transaction is unavailable.
    pub async fn with_house_static_source<C, F>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        context: HouseActionContext,
        position: [i32; 3],
        owner: &mut C,
        operation: F,
    ) -> Result<bool, DurabilityError>
    where
        C: Send,
        F: FnOnce(&mut C) + Send + 'static,
    {
        if position[2] != context.position[2]
            || position[0].abs_diff(context.position[0]) > 1
            || position[1].abs_diff(context.position[1]) > 1
        {
            return Ok(false);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run_with_context(owner,move|holder,deadline,owner|Box::pin(async move{
            let mut tx=crate::durability::db::begin_semantic_transaction(holder,deadline).await?;
            crate::durability::character_authority::assert_recovery_fence(&mut tx,&recovery).await?;
            crate::durability::db::lock_admission_relations(&mut tx).await?;
            if !prove_current_incarnation(&mut tx,&node).await?||!fenced(CURRENT,&node,&context).fetch_one(&mut *tx).await?.try_get::<bool,_>(0)?{return Ok(false)}
            let bytes:Vec<u8>=position.into_iter().flat_map(i32::to_be_bytes).collect();
            // This is the exact existing property tile authority, held through
            // publication. A nearby tile belonging to another House is refused.
            let tile:Option<i32>=sqlx::query_scalar("SELECT 1 FROM game_house_tiles WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND spatial_position=$3 FOR SHARE")
                .bind(context.house.world_id().as_bytes().as_slice()).bind(context.house.house_key()).bind(bytes).fetch_optional(&mut *tx).await?;
            if tile.is_none(){return Ok(false)}
            operation(owner);
            crate::durability::db::commit_semantic_transaction(tx,deadline).await?;
            Ok(true)
        })).await
    }
}
