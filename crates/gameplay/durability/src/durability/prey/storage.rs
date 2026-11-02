use super::super::{DurabilityError, item_mint_audit::Type2Transaction};
use oteryn_gameplay_progression::prey::PreyRules;
use oteryn_protocol_oteryn::prey::{Reply, Slot};
use sqlx::Row;

#[derive(Debug, Clone, Default)]
pub(super) struct Metrics {
    pub selected_at: Option<i64>,
    pub tier: u8,
    pub consumed: i64,
    pub wildcards: i64,
    pub free_rerolls: i64,
    pub gold_rerolls: i64,
    pub kills: i64,
    pub loot_gold: i64,
    pub pick_gold: i64,
}

impl Metrics {
    pub(super) fn start(&mut self, now: i64, tier: u8, wildcards: i64, pick_gold: i64) {
        let (free_rerolls, gold_rerolls) = (self.free_rerolls, self.gold_rerolls);
        *self = Self {
            selected_at: Some(now),
            tier,
            wildcards,
            free_rerolls,
            gold_rerolls,
            pick_gold,
            ..Self::default()
        };
    }
}

pub(super) async fn load(
    tx: &mut Type2Transaction<'_>,
    character: &[u8; 16],
    balance: u8,
    level: u32,
    expansion: bool,
    rules: &PreyRules,
) -> Result<(Reply, [Metrics; 3]), DurabilityError> {
    let rows = sqlx::query("SELECT * FROM game_character_prey_slots WHERE character_id=encode($1,'hex')::uuid ORDER BY slot LIMIT 3 FOR UPDATE").bind(character.as_slice()).fetch_all(&mut ***tx).await?;
    let allowed = rules
        .highest_allowed_tier(level)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut metrics = std::array::from_fn(|_| Metrics::default());
    let mut slots = Vec::new();
    for row in rows {
        let slot: i16 = row.try_get("slot")?;
        let i = usize::try_from(slot - 1).map_err(|_| DurabilityError::InvalidStoredState)?;
        if i >= 3 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let kind: Option<&str> = row.try_get("bonus_type")?;
        metrics[i] = Metrics {
            selected_at: row.try_get("selected_at_unix_ms")?,
            tier: row.try_get::<i16, _>("selected_tier")? as u8,
            consumed: row.try_get("consumed_combat_seconds")?,
            wildcards: row.try_get("wildcards_used")?,
            free_rerolls: row.try_get("list_rerolls_free")?,
            gold_rerolls: row.try_get("list_rerolls_gold")?,
            kills: row.try_get("chosen_kills")?,
            loot_gold: row.try_get("chosen_loot_gold")?,
            pick_gold: row.try_get("pick_again_gold")?,
        };
        slots.push(Slot {
            slot: slot as u8,
            state: match row.try_get::<&str, _>("state")? {
                "ACTIVE" => 2,
                "SELECTION" => 1,
                _ => 0,
            },
            creatures: row.try_get("creature_list")?,
            selected: row
                .try_get::<Option<String>, _>("creature_key")?
                .unwrap_or_default(),
            bonus: match kind {
                Some("DAMAGE") => 1,
                Some("REDUCTION") => 2,
                Some("XP") => 3,
                Some("LOOT") => 4,
                _ => 0,
            },
            grade: row.try_get::<Option<i16>, _>("grade")?.unwrap_or(0) as u8,
            percent: row.try_get::<Option<i16>, _>("bonus_percent")?.unwrap_or(0) as u8,
            hunting_seconds: row.try_get::<i32, _>("hunting_s_left")? as u32,
            option: match row.try_get::<&str, _>("option")? {
                "AUTO_REROLL" => 1,
                "LOCK" => 2,
                _ => 0,
            },
            free_reroll_at: row.try_get::<i64, _>("free_reroll_at")? as u64,
            chosen_tier: row
                .try_get::<Option<i16>, _>("chosen_tier")?
                .map_or(allowed, |v| v as u8),
            allowed_tier: allowed,
            selection_epoch: row.try_get::<i64, _>("selection_epoch")? as u64,
        });
    }
    for slot in 1..=3 {
        if !slots.iter().any(|s| s.slot == slot) {
            slots.push(Slot {
                slot,
                state: 0,
                creatures: vec![],
                selected: String::new(),
                bonus: 0,
                grade: 0,
                percent: 0,
                hunting_seconds: 0,
                option: 0,
                free_reroll_at: 0,
                chosen_tier: allowed,
                allowed_tier: allowed,
                selection_epoch: 0,
            });
        }
    }
    slots.sort_by_key(|s| s.slot);
    Ok((
        Reply {
            status: 0,
            wildcard_balance: balance,
            slots,
            list_size: rules.list_size(expansion) as u8,
            expansion_owned: expansion,
            offered_today: vec![],
            offered_next: String::new(),
        },
        metrics,
    ))
}

pub(super) async fn save(
    tx: &mut Type2Transaction<'_>,
    character: &[u8; 16],
    slot: &Slot,
    metrics: &Metrics,
) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_character_prey_slots(character_id,slot,state,creature_list,creature_key,bonus_type,grade,bonus_percent,hunting_s_left,option,free_reroll_at,slot_epoch,chosen_tier,selection_epoch,selected_at_unix_ms,selected_tier,consumed_combat_seconds,wildcards_used,list_rerolls_free,list_rerolls_gold,chosen_kills,chosen_loot_gold,pick_again_gold) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,1,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22) ON CONFLICT(character_id,slot) DO UPDATE SET state=excluded.state,creature_list=excluded.creature_list,creature_key=excluded.creature_key,bonus_type=excluded.bonus_type,grade=excluded.grade,bonus_percent=excluded.bonus_percent,hunting_s_left=excluded.hunting_s_left,option=excluded.option,free_reroll_at=excluded.free_reroll_at,slot_epoch=game_character_prey_slots.slot_epoch+1,chosen_tier=excluded.chosen_tier,selection_epoch=excluded.selection_epoch,selected_at_unix_ms=excluded.selected_at_unix_ms,selected_tier=excluded.selected_tier,consumed_combat_seconds=excluded.consumed_combat_seconds,wildcards_used=excluded.wildcards_used,list_rerolls_free=excluded.list_rerolls_free,list_rerolls_gold=excluded.list_rerolls_gold,chosen_kills=excluded.chosen_kills,chosen_loot_gold=excluded.chosen_loot_gold,pick_again_gold=excluded.pick_again_gold")
        .bind(character.as_slice()).bind(i16::from(slot.slot)).bind(if slot.state == 2 { "ACTIVE" } else { "SELECTION" })
        .bind(&slot.creatures).bind((!slot.selected.is_empty()).then_some(&slot.selected))
        .bind(match slot.bonus { 1 => Some("DAMAGE"), 2 => Some("REDUCTION"), 3 => Some("XP"), 4 => Some("LOOT"), _ => None })
        .bind((slot.grade > 0).then_some(i16::from(slot.grade))).bind((slot.bonus > 0).then_some(i16::from(slot.percent)))
        .bind(slot.hunting_seconds as i32).bind(match slot.option { 1 => "AUTO_REROLL", 2 => "LOCK", _ => "NONE" })
        .bind(slot.free_reroll_at as i64).bind(i16::from(slot.chosen_tier)).bind(slot.selection_epoch as i64)
        .bind(metrics.selected_at).bind(i16::from(metrics.tier)).bind(metrics.consumed).bind(metrics.wildcards)
        .bind(metrics.free_rerolls).bind(metrics.gold_rerolls).bind(metrics.kills).bind(metrics.loot_gold).bind(metrics.pick_gold)
        .execute(&mut ***tx).await?;
    Ok(())
}

pub(super) async fn record_end(
    tx: &mut Type2Transaction<'_>,
    character: &[u8; 16],
    occurrence: &[u8; 16],
    slot: &Slot,
    metrics: &Metrics,
    now: i64,
    reason: &str,
) -> Result<(), DurabilityError> {
    if slot.state != 2 {
        return Ok(());
    }
    sqlx::query("INSERT INTO game_character_prey_history(character_id,slot,selection_epoch,creature_key,tier,bonus_type,grade,selected_at_unix_ms,ended_at_unix_ms,consumed_combat_seconds,wildcards_used,list_rerolls_free,list_rerolls_gold,chosen_kills,chosen_loot_gold,pick_again_gold,end_reason,end_occurrence) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,encode($18,'hex')::uuid)")
        .bind(character.as_slice()).bind(i16::from(slot.slot)).bind(slot.selection_epoch as i64).bind(&slot.selected)
        .bind(i16::from(metrics.tier)).bind(i16::from(slot.bonus)).bind(i16::from(slot.grade)).bind(metrics.selected_at).bind(now)
        .bind(metrics.consumed).bind(metrics.wildcards).bind(metrics.free_rerolls).bind(metrics.gold_rerolls)
        .bind(metrics.kills).bind(metrics.loot_gold).bind(metrics.pick_gold).bind(reason).bind(occurrence.as_slice())
        .execute(&mut ***tx).await?;
    Ok(())
}

pub(super) async fn reset_epoch(
    tx: &mut Type2Transaction<'_>,
    world: &[u8; 16],
) -> Result<String, DurabilityError> {
    // Same committed daily boundary and serialization as Daily Reward; no wall-clock epoch.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:world-reset:' || (encode($1,'hex')::uuid)::text,0))")
        .bind(world.as_slice()).execute(&mut ***tx).await?;
    let epoch = sqlx::query_scalar::<_, String>("SELECT reset_epoch::text FROM public.game_world_reset_history WHERE world_id=encode($1,'hex')::uuid ORDER BY reset_epoch DESC LIMIT 1")
        .bind(world.as_slice()).fetch_optional(&mut ***tx).await?.ok_or(DurabilityError::InvalidStoredState)?;
    if epoch.parse::<u64>().ok().is_none_or(|v| v == 0) {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(epoch)
}

pub(super) async fn offered(
    tx: &mut Type2Transaction<'_>,
    character: &[u8; 16],
    epoch: &str,
    keys: &[String],
) -> Result<Vec<String>, DurabilityError> {
    // The ANY parameter bounds the result to the admitted catalogue, even after content retirement.
    Ok(sqlx::query_scalar("SELECT creature_key FROM game_character_prey_offered WHERE character_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND creature_key=ANY($3) ORDER BY creature_key LIMIT $4")
        .bind(character.as_slice()).bind(epoch).bind(keys).bind(keys.len() as i64).fetch_all(&mut ***tx).await?)
}

pub(super) async fn record_offers(
    tx: &mut Type2Transaction<'_>,
    character: &[u8; 16],
    occurrence: &[u8; 16],
    epoch: &str,
    keys: &[String],
) -> Result<(), DurabilityError> {
    for key in keys {
        let result = sqlx::query("INSERT INTO game_character_prey_offered(character_id,creature_key,reset_epoch,draw_occurrence) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,encode($4,'hex')::uuid) ON CONFLICT(character_id,creature_key) DO UPDATE SET reset_epoch=excluded.reset_epoch,draw_occurrence=excluded.draw_occurrence,created_xact_id=pg_current_xact_id() WHERE game_character_prey_offered.reset_epoch<excluded.reset_epoch")
            .bind(character.as_slice()).bind(key).bind(epoch).bind(occurrence.as_slice()).execute(&mut ***tx).await?;
        if result.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(())
}
