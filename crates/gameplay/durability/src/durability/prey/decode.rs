use super::super::DurabilityError;
use oteryn_gameplay_progression::prey::{
    BonusType, CREATURE_KEY_BYTES, HUNTING_SECONDS, LIST_SIZE, PreyBonus, PreyOption, SLOT_COUNT,
};
use sqlx::{Row, postgres::PgRow};

/// The caller holds the Character root; the bounded XP query selects active XP
/// rows only, after applying the existing live Account+World unlock predicate.
pub(in crate::durability) fn decode_xp_slot(
    row: &PgRow,
) -> Result<(i16, i64, PreyBonus), DurabilityError> {
    let invalid = || DurabilityError::InvalidStoredState;
    let slot: i16 = row.try_get("slot")?;
    let epoch: i64 = row.try_get("slot_epoch")?;
    let grade = u8::try_from(row.try_get::<i16, _>("grade")?).map_err(|_| invalid())?;
    let kind = BonusType::try_from(row.try_get::<&str, _>("bonus_type")?).map_err(|_| invalid())?;
    let bonus = PreyBonus::try_from((kind, grade)).map_err(|_| invalid())?;
    let percent: i16 = row.try_get("bonus_percent")?;
    let time: i32 = row.try_get("hunting_s_left")?;
    let creature: &str = row.try_get("creature_key")?;
    let list: Vec<String> = row.try_get("creature_list")?;
    let _: PreyOption =
        PreyOption::try_from(row.try_get::<&str, _>("option")?).map_err(|_| invalid())?;
    let deadline: i64 = row.try_get("free_reroll_at")?;
    if !(1..=i16::from(SLOT_COUNT)).contains(&slot)
        || epoch <= 0
        || kind != BonusType::Experience
        || row.try_get::<&str, _>("state")? != "ACTIVE"
        || percent != i16::from(bonus.percent)
        || !(1..=i64::from(HUNTING_SECONDS)).contains(&i64::from(time))
        || creature.is_empty()
        || creature.len() > CREATURE_KEY_BYTES
        || deadline < 0
        || list.len() > LIST_SIZE
        || list.iter().enumerate().any(|(index, key)| {
            key.is_empty() || key.len() > CREATURE_KEY_BYTES || list[..index].contains(key)
        })
    {
        return Err(invalid());
    }
    Ok((slot, epoch, bonus))
}
