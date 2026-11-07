//! Retained former-main-backpack format, proved by immutable named-House death history.
//! This classifies existing children only. Outer read/move owners still prove
//! current scope, custody, reach and every fresh session/Node permission.
use super::*;
pub(super) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    parent: [u8; 16],
) -> LoopResult<Vec<BackpackEntry>> {
    let direct:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)")
        .bind(parent.as_slice()).fetch_one(&mut **tx).await?;
    if !direct {
        return load_nested_entries(tx, parent).await;
    }
    let former_house_main:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_player_death_item_records r JOIN game_character_death_receipts d USING(death_occurrence_id) JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id AND i.world_id=r.world_id WHERE r.item_instance_id=encode($1,'hex')::uuid AND r.operation=1 AND r.source_slot=9 AND r.scope_kind=2 AND i.lifecycle=1 AND d.character_id=r.character_id AND d.death_world_id=r.world_id AND d.death_scope_kind=2 AND d.death_instance_id=r.instance_id AND EXISTS(SELECT 1 FROM game_house_transfer_receipts h WHERE h.character_id=r.character_id AND h.world_id=r.world_id AND h.direction=1 AND h.state=2 AND game_house_scope_instance_id(h.world_id,h.house_key)=r.instance_id))")
        .bind(parent.as_slice()).fetch_one(&mut **tx).await?;
    if !former_house_main {
        return load_nested_entries(tx, parent).await;
    }
    let mut entries = load_entries(tx, parent).await?;
    entries.extend(load_nested_entries(tx, parent).await?);
    if entries.len() > GAMEITEM01_CONTAINER_ENTRIES_MAX as usize {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let mut ordinals = std::collections::BTreeSet::new();
    let mut identities = std::collections::BTreeSet::new();
    if entries.iter().any(|entry| {
        !ordinals.insert(entry.placement_ordinal) || !identities.insert(entry.item.item_instance_id)
    }) {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.placement_ordinal));
    Ok(entries)
}
