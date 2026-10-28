use super::super::{DurabilityError, item_mint_audit::Type2Transaction};
use oteryn_protocol_oteryn::prey::{Reply, Slot};
use sqlx::Row;

pub(super) async fn load(
    tx: &mut Type2Transaction<'_>,
    character: &[u8; 16],
    access: &[bool; 3],
    balance: u8,
) -> Result<Reply, DurabilityError> {
    let rows = sqlx::query("SELECT * FROM game_character_prey_slots WHERE character_id=encode($1,'hex')::uuid ORDER BY slot LIMIT 3 FOR UPDATE").bind(character.as_slice()).fetch_all(&mut ***tx).await?;
    let mut slots = Vec::new();
    for row in rows {
        let slot: i16 = row.try_get("slot")?;
        let kind: Option<&str> = row.try_get("bonus_type")?;
        slots.push(Slot {
            slot: slot as u8,
            state: if !access[slot as usize - 1] {
                0
            } else {
                match row.try_get::<&str, _>("state")? {
                    "ACTIVE" => 2,
                    "SELECTION" => 1,
                    _ => 0,
                }
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
            });
        }
    }
    slots.sort_by_key(|s| s.slot);
    Ok(Reply {
        status: 0,
        wildcard_balance: balance,
        slots,
    })
}
pub(super) async fn save(
    tx: &mut Type2Transaction<'_>,
    character: &[u8; 16],
    slot: &Slot,
) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_character_prey_slots(character_id,slot,state,creature_list,creature_key,bonus_type,grade,bonus_percent,hunting_s_left,option,free_reroll_at,slot_epoch) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,1) ON CONFLICT(character_id,slot) DO UPDATE SET state=excluded.state,creature_list=excluded.creature_list,creature_key=excluded.creature_key,bonus_type=excluded.bonus_type,grade=excluded.grade,bonus_percent=excluded.bonus_percent,hunting_s_left=excluded.hunting_s_left,option=excluded.option,free_reroll_at=excluded.free_reroll_at,slot_epoch=game_character_prey_slots.slot_epoch+1")
    .bind(character.as_slice()).bind(i16::from(slot.slot)).bind(if slot.state==2 {"ACTIVE"} else {"SELECTION"}).bind(&slot.creatures).bind((!slot.selected.is_empty()).then_some(&slot.selected))
    .bind(match slot.bonus {1=>Some("DAMAGE"),2=>Some("REDUCTION"),3=>Some("XP"),4=>Some("LOOT"),_=>None}).bind((slot.grade>0).then_some(i16::from(slot.grade))).bind((slot.bonus>0).then_some(i16::from(slot.percent)))
    .bind(slot.hunting_seconds as i32).bind(match slot.option {1=>"AUTO_REROLL",2=>"LOCK",_=>"NONE"}).bind(slot.free_reroll_at as i64).execute(&mut ***tx).await?;
    Ok(())
}
