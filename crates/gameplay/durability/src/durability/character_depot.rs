//! DEPOT-1 storage owner (migration 0151).
//!
//! A town's seventeen compartments contain the existing native ItemInstances;
//! neither lockers nor chests are mintable items. Inbox delivery continues to
//! use `character_inbox` and migration 0076, independently of depot capacity.
//!
use super::item_transfer::InventoryItem;

/// Immediate durable custody. `town_id` preserves the map's u16 depot id,
/// including zero; World is supplied by the native item and Character root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterDepotEntry {
    pub item: InventoryItem,
    pub character_id: [u8; 16],
    pub world_id: [u8; 16],
    pub town_id: u16,
    pub box_number: u8,
    pub ordinal: u64,
}

/// Bounded page, newest first. The caller resolves a current reachable map locker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterDepotPage {
    pub total: u32,
    pub entries: Vec<CharacterDepotEntry>,
}

impl super::DurabilityRoot {
    /// Read the admitted Character's private compartment under current recovery authority.
    pub async fn read_depot_page(
        &self,
        recovery: &super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        character: crate::domain::CharacterId,
        town: u16,
        box_number: u8,
        page: u32,
    ) -> Result<CharacterDepotPage, super::DurabilityError> {
        use super::DurabilityError;
        use super::character_authority::assert_recovery_fence;
        use super::item_mint::{TypedDefinitionRef, uuid_text};
        use oteryn_gameplay_items::depot::{DEPOT_BOXES, DEPOT_HARD_ENTRIES, DEPOT_PAGE_ENTRIES};
        use sqlx::Row;
        if !(1..=DEPOT_BOXES).contains(&box_number)
            || page > (DEPOT_HARD_ENTRIES - 1) / DEPOT_PAGE_ENTRIES
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            // One statement gives the counter and page one MVCC snapshot. Runtime
            // needs no root UPDATE privilege just to browse an empty compartment.
            let rows = sqlx::query(
                "WITH summary AS (SELECT COALESCE(sum(committed),0)::bigint AS total \
                 FROM game_character_depot_box_counters WHERE character_id=encode($1,'hex')::uuid \
                   AND town_id=$2 AND box_number=$3) \
                 SELECT summary.total, page.* FROM summary LEFT JOIN LATERAL ( \
                   SELECT d.ordinal::text, d.world_id::text, i.item_instance_id::text, \
                          i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity \
                   FROM game_item_character_depot_locations d JOIN game_item_instances i USING (item_instance_id,world_id) \
                   WHERE d.character_id=encode($1,'hex')::uuid AND d.town_id=$2 AND d.box_number=$3 \
                   ORDER BY d.ordinal DESC LIMIT 32 OFFSET $4) page ON true ORDER BY page.ordinal::numeric DESC")
                .bind(character.as_bytes().as_slice()).bind(i32::from(town))
                .bind(i16::from(box_number)).bind(i64::from(page * DEPOT_PAGE_ENTRIES))
                .fetch_all(&mut *tx).await?;
            let total: i64 = rows.first().ok_or(DurabilityError::InvalidStoredState)?.try_get("total")?;
            let mut entries=Vec::with_capacity(rows.len());
            for row in rows {
                let Some(item_id)=row.try_get::<Option<String>,_>("item_instance_id")? else { continue; };
                entries.push(CharacterDepotEntry {
                    item: InventoryItem { item_instance_id: uuid_text(&item_id)?, definition: TypedDefinitionRef { family: row.try_get("definition_family")?, production_key: row.try_get("definition_production_key")?, revision_ref: row.try_get("definition_revision_ref")? }, quantity: u32::try_from(row.try_get::<i64,_>("quantity")?).map_err(|_| DurabilityError::InvalidStoredState)? },
                    character_id: *character.as_bytes(), world_id: uuid_text(row.try_get("world_id")?)?, town_id: town, box_number,
                    ordinal: row.try_get::<String,_>("ordinal")?.parse().map_err(|_| DurabilityError::InvalidStoredState)?,
                });
            }
            super::db::commit_semantic_transaction(tx,deadline).await?;
            Ok(CharacterDepotPage { total: u32::try_from(total).map_err(|_| DurabilityError::InvalidStoredState)?, entries })
        })).await
    }
}
