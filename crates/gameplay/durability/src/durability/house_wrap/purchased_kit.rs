//! Purchase provenance belongs to the existing furniture-wrap owner.
//! The NPC trade supplies its already minted kit inside that same transaction.
use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::item_mint::TypedDefinitionRef;
use super::super::{DurabilityError, DurabilityRoot};
use crate::domain::hireling::catalogue::wrapped_buy;

pub(crate) async fn record_purchased_kit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    transaction: [u8; 16],
    item: [u8; 16],
    source_row: usize,
    target: &TypedDefinitionRef,
) -> Result<(), DurabilityError> {
    let source=wrapped_buy(source_row).ok_or(DurabilityError::InvalidStoredState)?;
    if target.family!="Item" || target.production_key!=format!("oteryn:item.tibia.i{}",source.item_id)
        || target.revision_ref!="definition-r1" {return Err(DurabilityError::InvalidStoredState)}
    let row=i32::try_from(source_row).map_err(|_|DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_house_furniture_kit_origins(item_instance_id,mint_transaction_id,source_row,target_key,target_revision) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5)")
        .bind(item.as_slice()).bind(transaction.as_slice()).bind(row)
        .bind(&target.production_key).bind(&target.revision_ref).execute(&mut **tx).await?;
    Ok(())
}

impl DurabilityRoot {
    /// Read immutable origin metadata for content-pin validation. The existing wrap writer
    /// still rechecks current custody, House authority, ACL and this target under its locks.
    pub async fn house_furniture_kit_target(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        item: [u8; 16],
    ) -> Result<Option<TypedDefinitionRef>, DurabilityError> {
        let recovery=authority.record_for(self).map_err(|_|DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let target:Option<(String,String)>=sqlx::query_as("SELECT target_key,target_revision \
                FROM game_house_furniture_kit_origins WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(item.as_slice()).fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(target.map(|(production_key,revision_ref)| TypedDefinitionRef {family:"Item".into(),production_key,revision_ref}))
        })).await
    }
}
