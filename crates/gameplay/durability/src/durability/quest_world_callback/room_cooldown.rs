//! Crystal actions_portal_minis_kilmaresh.lua:87-106. Put the boss clock in
//! the same transaction as the admitted STEP, after historical receipt lookup.
use crate::domain::CharacterId;
use crate::durability::{
    DurabilityError,
    boss_encounter::cooldown_storage::{
        BossCooldownAdmissionData, BossCooldownChange, BossRewardKey, BossRewardStateError,
        set_boss_cooldowns_data,
    },
};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use sqlx::{Postgres, Transaction};

pub(in crate::durability) async fn admit(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    callback: &Callback,
    reset_epoch: u64,
) -> Result<bool, DurabilityError> {
    let Some(encounter) = &callback.effects.encounter else {
        return Ok(true);
    };
    let Some(seconds) = encounter
        .solo_timeout
        .and_then(|timeout| timeout.boss_cooldown_seconds)
    else {
        return Ok(true);
    };
    let [boss] = encounter.monsters.as_slice() else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let key = BossRewardKey::new(character, &boss.creature)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let (now, admission): (i64, Vec<u8>) = sqlx::query_as(
        "SELECT floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint,
                uuid_send(game_character_uuid_v7())",
    )
    .fetch_one(&mut **tx)
    .await?;
    let facts = BossCooldownAdmissionData {
        admission: admission
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        now,
        reset_epoch,
        cooldown_s: u64::from(seconds),
        boosted: false,
    };
    Ok(set_boss_cooldowns_data(tx, &[key], &facts)
        .await
        .map_err(|error| match error {
            BossRewardStateError::Unavailable(error) => error,
            _ => DurabilityError::InvalidStoredState,
        })?
        != BossCooldownChange::OnCooldown)
}
