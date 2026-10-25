//! Real auto-attack dispatch and the registered PostgreSQL durability path.
use super::party_xp::{self, Case};
use crate::bestiary_postgres_harness::{Harness, TestResult, debug};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::{ComposedFreshAdmission, actor_spell::TrainingSave};
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::time::Duration;

#[path = "../../../../../../apps/game-server/tests/support/stamina_postgres_cases.rs"]
mod persistence;

#[test]
fn hunting_stamina_postgres_real_attacks_idle_checkpoint_and_reload() -> TestResult {
    party_xp::run(Case::Stamina)
}

pub(super) async fn qualify(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    let (actor, session) = players[0];
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session(session)
            .await?;
    let item = crate::gameplay_transport::item_fence_of(current).ok_or("item")?;
    // Pre-hunting legacy stamina establishes the baseline at its first hunting receipt.
    sqlx::query("UPDATE game_character_build_state SET stamina_minutes=841 WHERE character_id=encode($1,'hex')::uuid")
        .bind(item.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
    let minutes = h
        .root
        .read_hunting_stamina(owner.character, item.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(minutes, 841);
    let creature;
    {
        let runtime = owner.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .load_stamina(&runtime, actor, session, minutes);
        assert!(owner.states.attack.lock().await.set_modes(
            &runtime,
            actor,
            session,
            oteryn_protocol_oteryn::attack::FightModes {
                fight_mode: oteryn_protocol_oteryn::attack::FightMode::Offensive,
                ..oteryn_protocol_oteryn::attack::FightModes::DEFAULT
            }
        ));
        let visible = runtime.visible_entities();
        let row = visible.creatures.first().ok_or("rat")?;
        creature = row.actor;
        let target = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: creature.placement_identity(),
            generation: row.generation,
        };
        assert_eq!(
            owner.states.attack.lock().await.set_target(
                &runtime,
                owner.rooms.gate_auto_attack(),
                actor,
                session,
                owner.owner_now(),
                Some(target),
                30
            ),
            oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        );
    }
    // Keep the content-owned rat alive between genuine swings; health is fixture setup,
    // while damage, hit qualification and the hunting signal use production dispatch.
    // Owner time is pinned to exactly 3s per swing, so the minutes spent depend on the swings
    // alone and never on how fast the runner executes them.
    const SWING_MICROS: u64 = 3_000_000;
    let start = owner.owner_now();
    for swing in 0..66 {
        owner.pinned_now = Some(SemanticTimeMicros::from_micros(
            start.get() + swing * SWING_MICROS,
        ));
        owner
            .runtime
            .lock()
            .await
            .set_test_creature_health(creature, 20)
            .map_err(debug)?;
        owner.refresh_attack_weapon(actor, session).await;
        owner.drain_auto_attacks().await;
    }
    // The real clock resumes one swing after the last pinned one.
    owner.clock_origin = std::time::Instant::now()
        .checked_sub(Duration::from_micros(start.get() + 66 * SWING_MICROS))
        .ok_or("clock")?;
    owner.pinned_now = None;
    let live = owner
        .states
        .spell_states
        .lock()
        .await
        .live_stamina(actor, session)
        .ok_or("live")?;
    assert_eq!(
        live, 838,
        "three minutes of real monster damage spend three stamina minutes"
    );
    assert_eq!(
        h.root
            .read_hunting_stamina(owner.character, item.character_id)
            .await
            .map_err(debug)?,
        841,
        "live until the checkpoint"
    );
    {
        let runtime = owner.runtime.lock().await;
        owner.states.attack.lock().await.set_target(
            &runtime,
            owner.rooms.gate_auto_attack(),
            actor,
            session,
            owner.owner_now(),
            None,
            31,
        );
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(3600))
        .ok_or("idle")?;
    owner.drain_auto_attacks().await;
    assert_eq!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .live_stamina(actor, session),
        Some(live),
        "idle owner passes spend nothing"
    );
    // The XP writer consumes the live penalty band before the checkpoint above 14h.
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .set_test_creature_health(creature, 1)
            .map_err(debug)?;
        let row = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|row| row.actor == creature)
            .ok_or("living rat")?;
        let target = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: creature.placement_identity(),
            generation: row.generation,
        };
        owner.states.attack.lock().await.set_target(
            &runtime,
            owner.rooms.gate_auto_attack(),
            actor,
            session,
            owner.owner_now(),
            Some(target),
            32,
        );
    }
    for _ in 0..32 {
        owner.refresh_attack_weapon(actor, session).await;
        owner.drain_auto_attacks().await;
        if owner
            .states
            .attack
            .lock()
            .await
            .kills()
            .has_session(session)
        {
            break;
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(3))
            .ok_or("clock")?;
    }
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    let xp:(i32,i64,i64) = sqlx::query_as("SELECT stamina_minutes,xp_awarded,raw_share FROM game_party_kill_xp_members WHERE character_id=encode($1,'hex')::uuid")
        .bind(item.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(xp.0, i32::from(live));
    assert!(xp.2 > 0);
    assert_eq!(
        xp.1,
        xp.2 / 2,
        "the live stamina penalty applies before checkpoint"
    );
    assert_eq!(
        owner.save_hunting_stamina(actor, session).await,
        TrainingSave::Saved
    );
    assert_eq!(
        h.root
            .read_hunting_stamina(owner.character, item.character_id)
            .await
            .map_err(debug)?,
        live
    );
    assert_eq!(
        owner.save_hunting_stamina(actor, session).await,
        TrainingSave::NotApplicable
    );
    let receipt: (Vec<u8>,i32,i32) = sqlx::query_as("SELECT uuid_send(build_occurrence_id),stamina_minutes_before,stamina_minutes_after FROM game_character_build_receipts WHERE character_id=encode($1,'hex')::uuid AND stamina_minutes_before<>stamina_minutes_after ORDER BY committed_character_revision DESC LIMIT 1")
        .bind(item.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((receipt.1, receipt.2), (841, 838));
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&receipt.0);
    let occurrence = crate::durability::character_build::BuildOccurrence::from_bytes(occurrence)
        .map_err(debug)?;
    let root = h
        .root
        .read_current_character(owner.character, item.character_id)
        .await
        .map_err(debug)?;
    let fence = crate::durability::character_progression::CurrentCharacterGameplayFence {
        character_id: item.character_id,
        game_session_id: session,
        connection_generation: item.connection_generation,
        character_lease_generation: item.character_lease_generation,
        runtime_scope: item.runtime_scope,
        scope_ownership_generation: item.scope_ownership_generation,
        expected_character_revision: root.revision,
    };
    let _slot = owner.revision_sequencer.acquire(item.character_id).await;
    assert!(matches!(
        h.root
            .commit_hunting_stamina(owner.character, owner.holder, fence, occurrence, (841, 838))
            .await
            .map_err(debug)?,
        crate::durability::character_build::BuildCommitOutcome::AlreadyCommitted(_)
    ));
    assert!(
        h.root
            .commit_hunting_stamina(owner.character, owner.holder, fence, occurrence, (841, 837))
            .await
            .is_err(),
        "conflicting occurrence cannot spend twice"
    );
    assert_eq!(
        h.root
            .read_hunting_stamina(owner.character, item.character_id)
            .await
            .map_err(debug)?,
        live
    );
    let mut stale = fence;
    stale.character_lease_generation += 1;
    let occurrence = crate::durability::character_build::BuildOccurrence::from_bytes(
        crate::bestiary_postgres_harness::id(200),
    )
    .map_err(debug)?;
    assert!(matches!(
        h.root
            .commit_hunting_stamina(
                owner.character,
                owner.holder,
                stale,
                occurrence,
                (live, live - 1)
            )
            .await,
        Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)
    ));
    for sql in [
        "UPDATE game_character_build_state SET stamina_minutes=0 WHERE character_id=encode($1,'hex')::uuid",
        "SELECT game_hunting_set_stamina(encode($1,'hex')::uuid,encode($1,'hex')::uuid,0)",
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        assert!(
            sqlx::query(sql)
                .bind(item.character_id.as_bytes().as_slice())
                .execute(&mut *tx)
                .await
                .is_err(),
            "runtime needs a same-transaction build receipt"
        );
        tx.rollback().await?;
    }
    Ok(())
}
