//! QUICKLOOT-1 durable Character preferences.
//!
//! These rows contain routing preferences only. Item custody remains in `item_transfer`; writes
//! reuse the Character transaction/fence without advancing gameplay progression.

use super::DurabilityError;
use super::character_progression::numeric_u64;
use oteryn_gameplay_items::quick_loot::{LootCategory, Recipient};
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableQuickLootSettings {
    pub category_targets: BTreeMap<LootCategory, Recipient<[u8; 16]>>,
    pub unassigned_target: Recipient<[u8; 16]>,
    pub fallback_to_main: bool,
    pub accept_only: bool,
    pub accepted_items: BTreeSet<String>,
    pub skipped_items: BTreeSet<String>,
}

impl Default for DurableQuickLootSettings {
    fn default() -> Self {
        Self {
            category_targets: BTreeMap::new(),
            unassigned_target: Recipient::MainContainer,
            fallback_to_main: true,
            accept_only: false,
            accepted_items: BTreeSet::new(),
            skipped_items: BTreeSet::new(),
        }
    }
}

pub(super) async fn store(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: crate::domain::CharacterId,
    next: u64,
    settings: DurableQuickLootSettings,
) -> std::result::Result<(), DurabilityError> {
    let unassigned = recipient_parts(settings.unassigned_target);
    sqlx::query("INSERT INTO game_character_quick_loot_settings(character_id,unassigned_container_id,unassigned_gold_pouch,fallback_to_main,committed_character_revision,accept_only) VALUES(encode($1,'hex')::uuid,CASE WHEN $2 IS NULL THEN NULL ELSE encode($2,'hex')::uuid END,$3,$4,$5::text::numeric(20,0),$6) ON CONFLICT(character_id) DO UPDATE SET unassigned_container_id=EXCLUDED.unassigned_container_id,unassigned_gold_pouch=EXCLUDED.unassigned_gold_pouch,fallback_to_main=EXCLUDED.fallback_to_main,committed_character_revision=EXCLUDED.committed_character_revision,accept_only=EXCLUDED.accept_only")
                .bind(character.as_bytes().as_slice()).bind(unassigned.0.as_ref().map(<[u8;16]>::as_slice)).bind(unassigned.1).bind(settings.fallback_to_main).bind(next.to_string()).bind(settings.accept_only).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM game_character_quick_loot_categories WHERE character_id=encode($1,'hex')::uuid").bind(character.as_bytes().as_slice()).execute(&mut **tx).await?;
    for (category, recipient) in settings.category_targets {
        let parts = recipient_parts(recipient);
        sqlx::query("INSERT INTO game_character_quick_loot_categories(character_id,category,container_id,gold_pouch) VALUES(encode($1,'hex')::uuid,$2,CASE WHEN $3 IS NULL THEN NULL ELSE encode($3,'hex')::uuid END,$4)")
                    .bind(character.as_bytes().as_slice()).bind(category_id(category)).bind(parts.0.as_ref().map(<[u8;16]>::as_slice)).bind(parts.1).execute(&mut **tx).await?;
    }
    Ok(())
}

fn category_id(value: LootCategory) -> i16 {
    match value {
        LootCategory::Gold => 0,
        LootCategory::CreatureProduct => 1,
        LootCategory::Equipment => 2,
        LootCategory::Consumable => 3,
        LootCategory::Other => 4,
    }
}
fn category(value: i16) -> std::result::Result<LootCategory, DurabilityError> {
    match value {
        0 => Ok(LootCategory::Gold),
        1 => Ok(LootCategory::CreatureProduct),
        2 => Ok(LootCategory::Equipment),
        3 => Ok(LootCategory::Consumable),
        4 => Ok(LootCategory::Other),
        _ => Err(DurabilityError::InvalidStoredState),
    }
}
fn recipient_parts(value: Recipient<[u8; 16]>) -> (Option<[u8; 16]>, bool) {
    match value {
        Recipient::Container(id) => (Some(id), false),
        Recipient::GoldPouch(id) => (Some(id), true),
        Recipient::MainContainer => (None, false),
    }
}
fn bytes(
    row: &sqlx::postgres::PgRow,
    name: &str,
) -> std::result::Result<Option<[u8; 16]>, DurabilityError> {
    let value: Option<Vec<u8>> = row.try_get(name)?;
    value
        .map(|v| {
            v.try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)
        })
        .transpose()
}
pub(super) async fn load(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: &[u8; 16],
) -> std::result::Result<DurableQuickLootSettings, DurabilityError> {
    let Some(head)=sqlx::query("SELECT CASE WHEN unassigned_container_id IS NULL THEN NULL ELSE decode(replace(unassigned_container_id::text,'-',''),'hex') END AS target,unassigned_gold_pouch,fallback_to_main,accept_only,committed_character_revision::text FROM game_character_quick_loot_settings WHERE character_id=encode($1,'hex')::uuid").bind(character.as_slice()).fetch_optional(&mut **tx).await? else{return Ok(DurableQuickLootSettings::default())};
    let _: u64 = numeric_u64(&head, "committed_character_revision")?;
    let id = bytes(&head, "target")?;
    let pouch: bool = head.try_get("unassigned_gold_pouch")?;
    let target = match (id, pouch) {
        (Some(id), true) => Recipient::GoldPouch(id),
        (Some(id), false) => Recipient::Container(id),
        (None, false) => Recipient::MainContainer,
        (None, true) => return Err(DurabilityError::InvalidStoredState),
    };
    let mut result = DurableQuickLootSettings {
        category_targets: BTreeMap::new(),
        unassigned_target: target,
        fallback_to_main: head.try_get("fallback_to_main")?,
        accept_only: head.try_get("accept_only")?,
        accepted_items: BTreeSet::new(),
        skipped_items: BTreeSet::new(),
    };
    for row in sqlx::query("SELECT category,CASE WHEN container_id IS NULL THEN NULL ELSE decode(replace(container_id::text,'-',''),'hex') END AS target,gold_pouch FROM game_character_quick_loot_categories WHERE character_id=encode($1,'hex')::uuid ORDER BY category").bind(character.as_slice()).fetch_all(&mut **tx).await?{
        let id=bytes(&row,"target")?; let pouch:bool=row.try_get("gold_pouch")?;
        let recipient=match(id,pouch){(Some(id),true)=>Recipient::GoldPouch(id),(Some(id),false)=>Recipient::Container(id),(None,false)=>Recipient::MainContainer,(None,true)=>return Err(DurabilityError::InvalidStoredState)};
        if result.category_targets.insert(category(row.try_get("category")?)?,recipient).is_some(){return Err(DurabilityError::InvalidStoredState)}
    }
    let limit = oteryn_gameplay_items::quick_loot::MAX_FILTER_ITEMS;
    for row in sqlx::query("SELECT accepted,item_production_key FROM game_character_quick_loot_filters WHERE character_id=encode($1,'hex')::uuid ORDER BY accepted,item_production_key LIMIT $2").bind(character.as_slice()).bind((limit*2+1) as i64).fetch_all(&mut **tx).await? {
        let list=if row.try_get("accepted")? { &mut result.accepted_items } else { &mut result.skipped_items };
        list.insert(row.try_get("item_production_key")?);
        if list.len()>limit { return Err(DurabilityError::InvalidStoredState); }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_ids_are_stable() -> std::result::Result<(), DurabilityError> {
        for (id, value) in [
            LootCategory::Gold,
            LootCategory::CreatureProduct,
            LootCategory::Equipment,
            LootCategory::Consumable,
            LootCategory::Other,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(category_id(value), id as i16);
            assert_eq!(category(id as i16)?, value);
        }
        Ok(())
    }
}
