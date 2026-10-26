//! INSPECT-1 bounded metadata read. This is never custody or mutation authority.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::imbuement::StoredImbuement;
use super::item_mint::TypedDefinitionRef;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::WorldId;
use sqlx::Row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemMetadata {
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
    pub tier: u8,
    pub imbuements: Vec<StoredImbuement>,
    pub contents: Vec<(TypedDefinitionRef, u32)>,
    pub charges: Option<u32>,
    pub remaining_ms: Option<u64>,
}
impl DurabilityRoot {
    /// Reads one live World item. Dispatch must establish and recheck current visibility
    /// or custody around this read; an arbitrary item identity grants no access.
    pub async fn read_inspection_metadata(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        world: WorldId,
        item: [u8; 16],
    ) -> Result<Option<ItemMetadata>, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &record).await?;
                    let result = read_metadata(&mut tx, world, item).await?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
}

async fn read_metadata(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    world: WorldId,
    item: [u8; 16],
) -> Result<Option<ItemMetadata>, DurabilityError> {
    // Lock the item against transfers/imbuement/tier writers until all metadata
    // has been read. Primary keys bound the item/tier and three imbuement rows.
    let row = sqlx::query("SELECT i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,COALESCE(t.tier,0)::smallint AS tier,ts.charges,CASE WHEN ts.deadline_at IS NULL THEN ts.remaining_ms ELSE GREATEST(0,ts.deadline_at-(extract(epoch FROM clock_timestamp())*1000)::bigint) END AS remaining_ms FROM game_item_instances i LEFT JOIN game_item_tiers t USING(item_instance_id) LEFT JOIN game_item_timed_states ts USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 FOR SHARE OF i")
                .bind(item.as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let result = if let Some(row) = row {
        let rows = sqlx::query("SELECT slot,category,tier,remaining_ms,imbuement_key,imbuement_revision FROM game_item_imbuements WHERE item_instance_id=encode($1,'hex')::uuid ORDER BY slot LIMIT 3")
                    .bind(item.as_slice()).fetch_all(&mut **tx).await?;
        let mut imbuements = Vec::new();
        for row in rows {
            let small = |name| -> Result<u8, DurabilityError> {
                u8::try_from(row.try_get::<i16, _>(name)?)
                    .map_err(|_| DurabilityError::InvalidStoredState)
            };
            let recipe = oteryn_gameplay_items::imbuement::imbuement_recipe(
                small("category")?
                    .checked_add(1)
                    .ok_or(DurabilityError::InvalidStoredState)?,
                small("tier")?,
            )
            .ok_or(DurabilityError::InvalidStoredState)?;
            if row.try_get::<String, _>("imbuement_key")? != recipe.key
                || row.try_get::<String, _>("imbuement_revision")?
                    != oteryn_gameplay_items::imbuement::IMBUEMENT_CATALOGUE_REVISION
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            imbuements.push(StoredImbuement {
                slot: small("slot")?,
                category: small("category")?,
                tier: small("tier")?,
                remaining_ms: u32::try_from(row.try_get::<i64, _>("remaining_ms")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            });
        }
        // An inspection is bounded even if a corrupt graph bypassed the item owner.
        let children=sqlx::query("WITH RECURSIVE tree(id,path,cycle) AS (SELECT encode($1,'hex')::uuid,ARRAY[encode($1,'hex')::uuid],false UNION ALL SELECT e.item_instance_id,t.path||e.item_instance_id,e.item_instance_id=ANY(t.path) FROM tree t JOIN (SELECT item_instance_id,parent_item_instance_id FROM game_item_container_entries UNION ALL SELECT item_instance_id,parent_item_instance_id FROM game_item_nested_entries) e ON e.parent_item_instance_id=t.id WHERE NOT t.cycle AND cardinality(t.path)<33) SELECT i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,t.cycle,cardinality(t.path) AS depth FROM tree t JOIN game_item_instances i ON i.item_instance_id=t.id WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 LIMIT 4097")
                    .bind(item.as_slice()).bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
        if children.len() > 4096 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let mut contents = Vec::new();
        for child in children {
            if child.try_get::<bool, _>("cycle")? || child.try_get::<i32, _>("depth")? > 32 {
                return Err(DurabilityError::InvalidStoredState);
            }
            contents.push((
                TypedDefinitionRef {
                    family: child.try_get("definition_family")?,
                    production_key: child.try_get("definition_production_key")?,
                    revision_ref: child.try_get("definition_revision_ref")?,
                },
                u32::try_from(child.try_get::<i64, _>("quantity")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            ));
        }
        Some(ItemMetadata {
            definition: TypedDefinitionRef {
                family: row.try_get("definition_family")?,
                production_key: row.try_get("definition_production_key")?,
                revision_ref: row.try_get("definition_revision_ref")?,
            },
            contents,
            charges: row
                .try_get::<Option<i32>, _>("charges")?
                .map(u32::try_from)
                .transpose()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            remaining_ms: row
                .try_get::<Option<i64>, _>("remaining_ms")?
                .map(u64::try_from)
                .transpose()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            tier: u8::try_from(row.try_get::<i16, _>("tier")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            imbuements,
        })
    } else {
        None
    };
    Ok(result)
}
