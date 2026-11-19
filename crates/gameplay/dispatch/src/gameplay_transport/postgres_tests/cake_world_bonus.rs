//! Crystal 201189f8, a_piece_of_cake_config.lua:116-118,456-467 and
//! a_piece_of_cake/world_bonus.lua:3-42. The persisted active occurrence
//! is a fixture. The timed-item owner and attack settlement are production.
use super::fixture::debug;
use super::{ComposedFreshAdmission, TestResult, party_xp};
use crate::domain::condition::{ConditionValues, ConflictKey};
use crate::foundation::runtime_actor_spell_types::{
    ManaShieldState, OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{CommandId, CommandRef, ExactActorRef, GameSessionId};

#[test]
fn cake_bonus_regenerates_both_players_and_scales_only_premium_shared_xp() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    party_xp::run(party_xp::Case::CakeBonus)
}

pub(super) async fn activate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pool: &sqlx::PgPool,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    insert_activation(pool, owner.world_id.as_bytes(), 2025, 1).await?;
    let [(actor, session), (target, target_session)] = players else {
        return Err("Cake bonus case requires two players".into());
    };
    let (actor, session, target, target_session) = (*actor, *session, *target, *target_session);
    let now = owner.owner_now();
    let staged = {
        let mut runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        states
            .get_mut(&runtime, actor, session)
            .ok_or("premium fixture actor")?
            .apply_owner_premium_transition(true, Some(owner.owner_now().get() + 600_000_000))
            .map_err(debug)?;
        let lease = runtime.bound_attacker_lease(actor, session).map_err(debug)?;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: lease.character_id(),
            current_lease_generation: lease.generation(),
            command: CommandRef::new(session, CommandId::new(105).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "cake-bonus-pending-target".into(),
                revisions: std::array::from_fn(|_| "fixture:1".into()),
            },
            binding: b"cake-bonus-pending-target".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1_000,
            effects: vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::ManaShield(ManaShieldState {
                    capacity: 50,
                    expires_ms: now.get() / 1_000 + 1_000,
                }),
            }],
            deferred: None,
        };
        let mut staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        runtime.reserve_spell_batch(&mut staged).map_err(debug)?;
        assert!(runtime.actor_spell_reserved(target));
        staged
    };
    owner.tick_timed_items().await;
    {
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let state = states.get(&runtime, target, target_session).ok_or("reserved target")?;
        assert!(
            state.owned_conditions().get(ConflictKey::Recovery).is_none(),
            "Cake must leave the reserved player's condition state untouched"
        );
        runtime.release_definitely_uncommitted_spell_batch(&staged).map_err(debug)?;
    }
    let retry_ms = owner.owner_now().get() / 1_000 + 1_000;
    owner.tick_timed_items_at(retry_ms).await;
    let sampled = owner.owner_now().get();
    let initial = check_regeneration(owner, players, sampled, 2025).await?;
    // Advance the containing item scheduler by one period without sleeping.
    owner.tick_timed_items_at(retry_ms + 1_000).await;
    assert_eq!(
        check_regeneration(owner, players, sampled, 2025).await?,
        initial,
        "an active world reward keeps its condition identity and expiry",
    );
    // Crystal a_piece_of_cake_config.lua:456-467 applies a fresh seven-day
    // reward to online players at completion. Give the fixture a newer award
    // while both actors still carry a shorter Cake condition from the prior one.
    insert_activation(pool, owner.world_id.as_bytes(), 2026, 0).await?;
    owner.tick_timed_items_at(retry_ms + 2_000).await;
    let renewed = check_regeneration(owner, players, owner.owner_now().get(), 2026).await?;
    for ((before_sequence, before_ms), (sequence, remaining_ms)) in initial.into_iter().zip(renewed) {
        assert_ne!(sequence, before_sequence);
        assert!(remaining_ms > before_ms);
    }
    Ok(())
}

async fn insert_activation(
    pool: &sqlx::PgPool,
    world: &[u8; 16],
    year: i32,
    elapsed_days: i32,
) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_cake_world_progress VALUES(encode($1,'hex')::uuid,$2,200,200)")
        .bind(world.as_slice())
        .bind(year)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_cake_phases(world_id,event_year,stage,layer,bonus_activated_at,bonus_until)
         SELECT encode($1,'hex')::uuid,$2,3,4,activated,activated+interval '604800 seconds'
         FROM (SELECT transaction_timestamp()-$3::integer*interval '1 day' AS activated) AS bonus",
    )
    .bind(world.as_slice())
    .bind(year)
    .bind(elapsed_days)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn check_regeneration(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &[(ExactActorRef, GameSessionId)],
    sampled: u64,
    year: i32,
) -> TestResult<Vec<(u32, i32)>> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let now = owner.owner_now().get();
    let mut snapshots = Vec::new();
    for (&(actor, session), (health_interval_ms, mana_interval_ms)) in
        players.iter().zip([(8_000, 4_000), (6_000, 6_000)])
    {
        let condition = states
            .get(&runtime, actor, session)
            .ok_or("player state")?
            .owned_conditions()
            .active_at(ConflictKey::Recovery, now)
            .ok_or("world regeneration missing after scheduler tick")?;
        assert_eq!(
            condition.definition().key(),
            format!("source.cake.world-regeneration.{year}")
        );
        let ConditionValues::SpellRegeneration {
            health_gain,
            health_interval_ms: health_interval,
            mana_gain,
            mana_interval_ms: mana_interval,
            ..
        } = condition.definition().values() else {
            return Err("world reward must project typed regeneration".into());
        };
        // The bonus is 25% of the vocation's flat native gain, rounded down
        // with a minimum of one point for each pool with a positive rate.
        assert_eq!((health_gain, mana_gain), (1, 1));
        assert_eq!((health_interval, mana_interval), (health_interval_ms, mana_interval_ms));
        let remaining_ms = condition.remaining_duration_ms(sampled).ok_or("bonus expiry")?;
        assert!((1..=604_800_000).contains(&remaining_ms));
        snapshots.push((condition.sequence(), remaining_ms));
    }
    assert_eq!(snapshots.len(), 2);
    Ok(snapshots)
}

pub(super) fn check_awards(premium: i64, free: i64) {
    // Four raw XP each, doubled at level one. Premium stamina adds 50%,
    // then the Cake callback adds another floor(50%) to the premium award.
    assert_eq!((premium, free), (18, 8));
}
