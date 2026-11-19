use sqlx::Row;
use super::*;

use oteryn_protocol_oteryn::prey as wire;

#[derive(Default)]
pub(super) struct HistoryCounters {
    pub(super) consumed: i64,
    pub(super) wildcards: i64,
    pub(super) free_rerolls: i64,
    pub(super) gold_rerolls: i64,
    pub(super) kills: i64,
    pub(super) loot_gold: i64,
    pub(super) pick_gold: i64,
}

pub(super) async fn selected_at(h: &Harness, slot: u8) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT selected_at_unix_ms FROM game_character_prey_slots WHERE character_id=encode($1,'hex')::uuid AND slot=$2")
        .bind(id(41).as_slice()).bind(i16::from(slot)).fetch_one(&h.pool).await?)
}

pub(super) async fn assert_history(
    h: &Harness, slot: &wire::Slot, catalogue: &crate::durability::prey::PreyCatalogue,
    started: i64, reason: &str, counters: HistoryCounters,
) -> TestResult {
    let row = sqlx::query("SELECT h.*,uuid_send(h.character_id) AS character_bytes,h.created_xact_id=r.created_xact_id AS atomic_receipt FROM game_character_prey_history h JOIN game_character_prey_receipts r ON r.character_id=h.character_id AND r.occurrence=h.end_occurrence WHERE h.character_id=encode($1,'hex')::uuid AND h.slot=$2 AND h.selection_epoch=$3")
        .bind(id(41).as_slice()).bind(i16::from(slot.slot)).bind(slot.selection_epoch as i64)
        .fetch_one(&h.pool).await?;
    assert_eq!(row.try_get::<Vec<u8>,_>("character_bytes")?, id(41));
    assert_eq!(row.try_get::<i16,_>("slot")?, i16::from(slot.slot));
    assert_eq!(row.try_get::<i64,_>("selection_epoch")?, slot.selection_epoch as i64);
    assert_eq!(row.try_get::<String,_>("creature_key")?, slot.selected);
    assert_eq!(row.try_get::<i16,_>("tier")?, i16::from(catalogue.rules().creatures.get(&slot.selected).ok_or("selected tier")?.tier));
    assert_eq!(row.try_get::<i16,_>("bonus_type")?, i16::from(slot.bonus));
    assert_eq!(row.try_get::<i16,_>("grade")?, i16::from(slot.grade));
    assert_eq!(row.try_get::<Option<i64>,_>("selected_at_unix_ms")?, Some(started));
    let ended: i64 = row.try_get("ended_at_unix_ms")?;
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
        .fetch_one(&h.pool).await?;
    assert!((started..=now).contains(&ended));
    assert_eq!(row.try_get::<String,_>("end_reason")?, reason);
    assert!(row.try_get::<bool,_>("atomic_receipt")?);
    for (field, expected) in [
        ("consumed_combat_seconds", counters.consumed),
        ("wildcards_used", counters.wildcards),
        ("list_rerolls_free", counters.free_rerolls),
        ("list_rerolls_gold", counters.gold_rerolls),
        ("chosen_kills", counters.kills),
        ("chosen_loot_gold", counters.loot_gold),
        ("pick_again_gold", counters.pick_gold),
    ] { assert_eq!(row.try_get::<i64,_>(field)?, expected, "{field} for selection {}", slot.selection_epoch); }
    Ok(())
}
