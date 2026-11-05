//! Bounded owner reads for a complete source program, within its root-locked transaction.
use super::*;
use oteryn_gameplay_quests::quest::npc_rules::{NpcInventorySnapshot, NpcItemKey};
use sqlx::Row;

pub(super) async fn inventory(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: super::super::character_progression::CurrentCharacterGameplayFence,
) -> Result<NpcInventorySnapshot, DurabilityError> {
    let world = fence.runtime_scope.world_id();
    let rows = sqlx::query(
        "SELECT i.item_instance_id::text,i.definition_production_key AS item,i.quantity, \
         f.fluid_subtype,c.kind,c.location FROM \
         game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c \
         JOIN game_item_instances i ON i.item_instance_id=c.item \
         LEFT JOIN game_item_fluid_states f USING(item_instance_id) \
         WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 \
         AND i.definition_family='Item' AND i.quantity>0 \
         ORDER BY i.item_instance_id LIMIT 4097 FOR UPDATE OF i",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(world.as_bytes().as_slice())
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > 4096 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let backpack_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_item_container_slots \
         WHERE character_id=encode($1,'hex')::uuid)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if backpack_exists
        && !rows.iter().any(|row| {
            row.try_get::<String, _>("kind")
                .is_ok_and(|kind| kind == "backpack")
        })
    {
        // The existing carried owner refuses its entire bag tree at its depth/
        // count bound. That refusal must never turn into an empty inventory.
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut snapshot = NpcInventorySnapshot::default();
    for row in rows {
        let id: String = row.try_get("item_instance_id")?;
        if !seen.insert(id) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let item: String = row.try_get("item")?;
        let quantity = u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let subtype = row
            .try_get::<Option<i16>, _>("fluid_subtype")?
            .map(u16::try_from)
            .transpose()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        use crate::domain::currency::Coin;
        let unit = if item == Coin::Gold.production_key() {
            1_u64
        } else if item == Coin::Platinum.production_key() {
            100
        } else if item == Coin::Crystal.production_key() {
            10_000
        } else {
            0
        };
        snapshot.money = snapshot
            .money
            .checked_add(
                unit.checked_mul(u64::from(quantity))
                    .ok_or(DurabilityError::InvalidStoredState)?,
            )
            .ok_or(DurabilityError::InvalidStoredState)?;
        let kind: String = row.try_get("kind")?;
        let location: serde_json::Value = row.try_get("location")?;
        let slot = if kind == "backpack" {
            Some("backpack")
        } else if kind == "equipment" {
            match location.get("slot").and_then(serde_json::Value::as_u64) {
                Some(1) => Some("head"),
                Some(2) => Some("necklace"),
                Some(3) => Some("backpack"),
                Some(4) => Some("armor"),
                Some(5) => Some("right"),
                Some(6) => Some("left"),
                Some(7) => Some("legs"),
                Some(8) => Some("feet"),
                Some(9) => Some("ring"),
                Some(10) => Some("ammo"),
                _ => return Err(DurabilityError::InvalidStoredState),
            }
        } else {
            None
        };
        if let Some(slot) = slot {
            snapshot.equipped.insert(slot.into(), item.clone());
        }
        let count = snapshot
            .items
            .entry(NpcItemKey { item, subtype })
            .or_default();
        *count = count
            .checked_add(quantity)
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    let balance: Option<String> = sqlx::query_scalar(
        "SELECT b.balance::text FROM game_account_bank_balances b \
         JOIN game_character_roots r USING(account_id) \
         WHERE r.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid FOR UPDATE OF b",
    ).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    snapshot.bank = balance
        .map(|balance| balance.parse::<u64>())
        .transpose()
        .map_err(|_| DurabilityError::InvalidStoredState)?
        .unwrap_or(0);
    Ok(snapshot)
}
