//! The existing World ledger, with one immutable phase projection per server save.
use super::*;
use crate::foundation::CommandRef;

// Fandom Steamship World Change/Spoiler: 200 coal and the next save starts the working week.
const SHIP_COAL: u64 = 200;
const SHIP_WORKING_WEEK: i16 = 7;
// TibiaWiki BR Awash World Change: 202 coal shared by the three pumps.
const PUMP_COAL: u64 = 202;

#[derive(Default, sqlx::FromRow)]
struct Save {
    ship_carry: i64,
    ship_saves_remaining: i16,
    pump_carry: i64,
    awash_drained: bool,
}

pub(super) async fn current_epoch(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
) -> Result<Option<u64>, DurabilityError> {
    lock_admission_relations(tx).await?;
    let current: Option<String> = sqlx::query_scalar(
        "SELECT a.reset_epoch::text FROM game_world_reset_active a
         WHERE a.world_id=encode($1,'hex')::uuid AND NOT EXISTS(
             SELECT 1 FROM game_world_reset_jobs j WHERE j.world_id=a.world_id AND j.phase='RETIRING')",
    )
    .bind(world.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    current.map(|value| value.parse().map_err(|_| DurabilityError::InvalidStoredState)).transpose()
}

async fn read_save(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<Save, DurabilityError> {
    Ok(sqlx::query_as(
        "SELECT ship_carry,ship_saves_remaining,pump_carry,awash_drained
         FROM game_world_contribution_days WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric",
    )
    .bind(world.as_bytes().as_slice())
    .bind(epoch.to_string())
    .fetch_optional(&mut **tx)
    .await?
    .unwrap_or_default())
}

pub(super) async fn record_save(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<(), DurabilityError> {
    let Some(previous) = epoch.checked_sub(1) else {
        return Ok(());
    };
    let mut save = read_save(tx, world, previous).await?;
    let ship = stock(tx, world, previous, ContributionKind::Coal, save.ship_carry).await?;
    save.ship_saves_remaining = (save.ship_saves_remaining - 1).max(0);
    if save.ship_saves_remaining == 0 && ship >= SHIP_COAL {
        save.ship_saves_remaining = SHIP_WORKING_WEEK;
        save.ship_carry = 0;
    } else {
        // Surplus beyond the next working week cannot fund another activation.
        save.ship_carry =
            i64::try_from(ship.min(SHIP_COAL)).map_err(|_| DurabilityError::InvalidStoredState)?;
    }

    let pump = stock(tx, world, previous, ContributionKind::AwashCoal, save.pump_carry).await?;
    save.awash_drained = pump >= PUMP_COAL;
    save.pump_carry = if save.awash_drained {
        0
    } else {
        i64::try_from(pump).map_err(|_| DurabilityError::InvalidStoredState)?
    };
    sqlx::query(
        "INSERT INTO game_world_contribution_days
         (world_id,reset_epoch,ship_carry,ship_saves_remaining,pump_carry,awash_drained)
         VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,$5,$6)",
    )
    .bind(world.as_bytes().as_slice())
    .bind(epoch.to_string())
    .bind(save.ship_carry)
    .bind(save.ship_saves_remaining)
    .bind(save.pump_carry)
    .bind(save.awash_drained)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn stock(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    kind: ContributionKind,
    carry: i64,
) -> Result<u64, DurabilityError> {
    let carry = u64::try_from(carry).map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(carry.saturating_add(tally(tx, world, epoch, kind).await?))
}

pub(in crate::durability) async fn gate(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    key: &str,
) -> Result<Option<bool>, DurabilityError> {
    if !matches!(key, "steamship" | "awash" | "awash_pumping" | "swamp_fever_unmedicated" | "swamp_fever_medicine_delivered") {
        return Ok(None);
    }
    let Some(epoch) = current_epoch(tx, world).await? else {
        return Ok(Some(false));
    };
    let active = match key {
        // These are delivery phases. Sources do not give the epidemic suppression target.
        "swamp_fever_unmedicated" => tally(tx, world, epoch, ContributionKind::Medicine).await? == 0,
        "swamp_fever_medicine_delivered" => tally(tx, world, epoch, ContributionKind::Medicine).await? > 0,
        "steamship" => read_save(tx, world, epoch).await?.ship_saves_remaining > 0,
        "awash" => read_save(tx, world, epoch).await?.awash_drained,
        _ => {
            let save = read_save(tx, world, epoch).await?;
            !save.awash_drained
                && stock(tx, world, epoch, ContributionKind::AwashCoal, save.pump_carry).await? >= PUMP_COAL
        }
    };
    Ok(Some(active))
}

/// Called after the exact source debit, before the caller commits its quest receipt.
pub(in crate::durability) async fn credit(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    world: WorldId,
    expected_epoch: Option<u64>,
    command: CommandRef,
    kind: ContributionKind,
    amount: u32,
) -> Result<bool, DurabilityError> {
    let Some(epoch) = current_epoch(tx, world).await? else {
        return Ok(false);
    };
    if expected_epoch.is_some_and(|expected| expected != epoch) {
        return Ok(false);
    }
    let mut hash = Sha256::new();
    hash.update(b"oteryn:world-contribution-command:v1");
    hash.update(command.game_session_id().as_bytes());
    hash.update(command.command_id().get().to_be_bytes());
    let mut operation = [0; 16];
    operation.copy_from_slice(&hash.finalize()[..16]);
    // The caller already checked its exact Quest receipt before spending. A ledger
    // replay here would describe a different operation and must roll back the debit.
    Ok(matches!(
        contribute(tx, recovery, world, epoch, operation, kind, amount).await?,
        ContributionWrite::Applied(_)
    ))
}

/// Runs alongside the ordinary NPC reward or selected World USE debit.
pub(in crate::durability) async fn credit_quest(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    cause: crate::durability::quest_state::QuestCause,
    reward: Option<&oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback>,
    callback: Option<(&oteryn_gameplay_quests::quest::world_callbacks::Callback, u64)>,
) -> Result<bool, DurabilityError> {
    use crate::durability::quest_state::QuestCause;
    use oteryn_gameplay_quests::quest::world_contributions::{medicine, pump_coal, ship_coal};

    let world = fence.runtime_scope.world_id();
    if let Some((callback, epoch)) = callback
        && let Some(amount) = pump_coal(callback)
    {
        let QuestCause::Use(command) = cause else {
            return Ok(false);
        };
        return credit(tx, recovery, world, Some(epoch), command, ContributionKind::AwashCoal, amount).await;
    }
    if let Some((kind, amount)) = reward.and_then(|callback| {
        ship_coal(callback).map(|amount| (ContributionKind::Coal, amount))
            .or_else(|| medicine(callback).map(|amount| (ContributionKind::Medicine, amount)))
    }) {
        let QuestCause::Command(command) = cause else {
            return Ok(false);
        };
        return credit(tx, recovery, world, None, command, kind, amount).await;
    }
    Ok(true)
}
