//! Crystal 201189f8, a_piece_of_cake_config.lua:116-118,456-467 and
//! a_piece_of_cake/world_bonus.lua:3-42. The persisted active occurrence
//! is a fixture. The timed-item owner and attack settlement are production.
use super::fixture::{Harness, debug};
use super::{ComposedFreshAdmission, TestResult, party_xp};
use crate::domain::condition::ConflictKey;
use crate::foundation::{ExactActorRef, GameSessionId};

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
    harness: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    let world = owner.world_id.as_bytes();
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_cake_world_progress VALUES(encode($1,'hex')::uuid,2026,200,200)")
        .bind(world.as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_cake_phases(world_id,event_year,stage,layer,bonus_activated_at,bonus_until)
         VALUES(encode($1,'hex')::uuid,2026,3,4,transaction_timestamp(),
                transaction_timestamp()+interval '604800 seconds')",
    ).bind(world.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let (actor, session) = players[0];
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        states
            .get_mut(&runtime, actor, session)
            .ok_or("premium fixture actor")?
            .apply_owner_premium_transition(true, Some(owner.owner_now().get() + 600_000_000))
            .map_err(debug)?;
    }
    owner.tick_timed_items().await;
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    for &(actor, session) in players {
        let condition = states
            .get(&runtime, actor, session)
            .ok_or("player state")?
            .owned_conditions()
            .active_at(ConflictKey::Recovery, owner.owner_now().get())
            .ok_or("world regeneration missing after scheduler tick")?;
        assert_eq!(
            condition.definition().key(),
            "source.cake.world-regeneration"
        );
    }
    Ok(())
}

pub(super) fn check_awards(premium: i64, free: i64) {
    // Four raw XP each, doubled at level one. Premium stamina adds 50%,
    // then the Cake callback adds another floor(50%) to the premium award.
    assert_eq!((premium, free), (18, 8));
}
