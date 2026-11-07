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
    let item_id = wrapped_buy(source_row)
        .map(|source| source.item_id)
        .or_else(|| {
            u32::try_from(source_row)
                .ok()
                .and_then(oteryn_gameplay_items::furniture::npc_offer_for_row)
                .map(|source| source.item_id)
        })
        .ok_or(DurabilityError::InvalidStoredState)?;
    if target.family != "Item"
        || target.production_key != format!("oteryn:item.tibia.i{}", item_id)
        || target.revision_ref != "definition-r1"
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let row = i32::try_from(source_row).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_house_furniture_kit_origins(item_instance_id,mint_transaction_id,source_row,target_key,target_revision) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5)")
        .bind(item.as_slice()).bind(transaction.as_slice()).bind(row)
        .bind(&target.production_key).bind(&target.revision_ref).execute(&mut **tx).await?;
    Ok(())
}

/// Metadata sealed by the actual prior wrap or purchased-kit origin.
/// Its current wrapper key is server-read, never a client-provided unwrap target.
#[derive(Clone, Debug)]
pub struct HouseFurnitureKitSource {
    pub target: TypedDefinitionRef,
    pub wrapper: TypedDefinitionRef,
}
impl DurabilityRoot {
    pub async fn house_furniture_kit_source(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        item: [u8; 16],
    ) -> Result<Option<HouseFurnitureKitSource>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let source:Option<(String,String,String,String)>=sqlx::query_as("WITH current_item AS (SELECT * FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1 AND quantity=1 AND definition_family='Item') SELECT target_key,target_revision,wrapper_key,wrapper_revision FROM (SELECT r.before_item->>'definition_production_key' AS target_key,r.before_item->>'definition_revision_ref' AS target_revision,i.definition_production_key AS wrapper_key,i.definition_revision_ref AS wrapper_revision,1 AS priority,(r.before_item->>'state_revision')::numeric AS ordering FROM game_house_furniture_wrap_receipts r JOIN current_item i USING(item_instance_id,world_id) WHERE r.action=1 AND r.after_key=i.definition_production_key AND r.before_item->>'definition_family'='Item' UNION ALL SELECT o.target_key,o.target_revision,i.definition_production_key,i.definition_revision_ref,2,0 FROM game_house_furniture_kit_origins o JOIN current_item i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i23398') sources ORDER BY priority,ordering DESC LIMIT 1")
                .bind(item.as_slice()).fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(source.map(|(key,revision,wrapper,wrapper_revision)|HouseFurnitureKitSource{
                target:TypedDefinitionRef{family:"Item".into(),production_key:key,revision_ref:revision},
                wrapper:TypedDefinitionRef{family:"Item".into(),production_key:wrapper,revision_ref:wrapper_revision},
            }))
        })).await
    }
}

/// The exact immutable producer controls wording; neither client nor mutable attributes do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FurnitureKitDescriptionKind {
    Wrapped,
    Purchased,
}
#[derive(Clone, Debug)]
pub struct FurnitureKitDescriptionSource {
    pub target: TypedDefinitionRef,
    pub kind: FurnitureKitDescriptionKind,
}
impl DurabilityRoot {
    pub async fn furniture_kit_description_source(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        item: [u8; 16],
    ) -> Result<Option<FurnitureKitDescriptionSource>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let target:Option<(String,String,String,i32)>=sqlx::query_as("WITH live_kit AS (SELECT * FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1 AND quantity=1 AND definition_family='Item' AND definition_production_key='oteryn:item.tibia.i23398') SELECT family,key,revision,kind FROM (SELECT r.before_item->>'definition_family' AS family,r.before_item->>'definition_production_key' AS key,r.before_item->>'definition_revision_ref' AS revision,1 AS kind,(r.before_item->>'state_revision')::numeric AS ordering FROM game_house_furniture_wrap_receipts r JOIN live_kit i USING(item_instance_id,world_id) WHERE r.action=1 AND r.after_key=i.definition_production_key UNION ALL SELECT 'Item',o.target_key,o.target_revision,2,0 FROM game_house_furniture_kit_origins o JOIN live_kit i USING(item_instance_id) JOIN game_house_furniture_kit_catalogue c USING(source_row) JOIN game_npc_trades trade ON trade.transaction_id=o.mint_transaction_id WHERE o.source_row<2000000 AND o.target_key=c.target_key AND o.target_revision=i.definition_revision_ref AND (c.npc_key IS NULL OR c.npc_key=trade.npc_key)) sources ORDER BY kind,ordering DESC LIMIT 1")
                .bind(item.as_slice()).fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;
            target.map(|(family,production_key,revision_ref,kind)|Ok(FurnitureKitDescriptionSource{
                target:TypedDefinitionRef{family,production_key,revision_ref},
                kind:match kind {1=>FurnitureKitDescriptionKind::Wrapped,2=>FurnitureKitDescriptionKind::Purchased,_=>return Err(DurabilityError::InvalidStoredState)},
            })).transpose()
        })).await
    }
}
