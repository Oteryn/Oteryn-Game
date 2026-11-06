use super::*;

/// A real native player lethal arrives after the Boss lethal and before the
/// first encounter cadence that could otherwise grant its reward.
pub(super) async fn probe_boss_death_order(
    h: &Harness,
    driver: &ComposedFreshAdmission<'_, '_, '_>,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    destination: AdmittedSession,
    before_xp: i64,
    actual_bosses: i64,
    now: u64,
) -> TestResult {
    let began = std::time::Instant::now();
    let actor = destination.runtime_actor.ok_or("death-order actor")?;
    let character = destination
        .item_fence
        .ok_or("death-order fence")?
        .character_id;
    assert!(
        actual_bosses > 0,
        "death-order PG proof requires an actual physical Boss lethal"
    );
    let still_uncredited: i64 = sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        still_uncredited, before_xp,
        "no global tick has settled the actual Boss lethal"
    );
    // The fixture has no background holder maintainer. Qualify the retained
    // origin's actual pinned temple before asking the real death owner to use it.
    super::super::maintain_ready(owner.root).await?;
    let temple = driver.runtime.lock().await.respawn_position();
    assert_eq!(
        temple,
        MovementLocalPosition {
            x: 32375,
            y: 31158,
            floor: 8
        }
    );
    let server = driver
        .runtime
        .lock()
        .await
        .content_pin()
        .server_artifact_digest();
    {
        let doors = driver.door.lock().await;
        assert!(
            crate::gameplay_transport::respawn_cell_admissible(
                driver.movement_cells.index(),
                driver.movement_cells.scope(),
                driver.world_id,
                server,
                doors.blocking_cells(),
                temple,
            ),
            "fixture origin temple is qualified ground under its actual source pin"
        );
    }
    assert_eq!(
        owner.home_temple(destination.game_session_id).await,
        Some(temple),
        "current Instance resolves the qualified retained origin temple"
    );
    let death = {
        let runtime = owner.runtime.lock().await;
        let mut spells = owner.states.spell_states.lock().await;
        spells
            .apply_creature_damage_native(
                &runtime,
                actor,
                destination.game_session_id,
                1_000_000,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now + 2_000),
            )
            .ok_or("authoritative native player lethal")?;
        spells
            .player_death(&runtime, actor, destination.game_session_id)
            .ok_or("native PlayerDeath")?
    };
    let settle = async {
        loop {
            let result = owner
                .settle_death(actor, destination.game_session_id, death, false)
                .await
                .0;
            if result != crate::gameplay_transport::SessionDeath::Retry {
                return result;
            }
            let observed = tokio::time::Instant::now();
            owner
                .root
                .wait_holder_retry(observed, observed + std::time::Duration::from_millis(25))
                .await;
        }
    };
    let settled = match tokio::time::timeout(std::time::Duration::from_secs(5), settle).await {
        Ok(settled) => settled,
        Err(error) => {
            let (xp, receipts): (i64, i64) = sqlx::query_as("SELECT total_experience,(SELECT count(*) FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid) FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            let pending = owner
                .states
                .attack
                .lock()
                .await
                .kills()
                .pending_soulpit_rewards(destination.game_session_id)
                .len();
            let training = {
                let runtime = owner.runtime.lock().await;
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .has_training_to_flush(&runtime, actor, destination.game_session_id)
            };
            eprintln!(
                "SV8_SOULPIT_DEATH_ORDER_TIMEOUT xp={xp} death_receipts={receipts} pending_boss_rewards={pending} training_to_flush={training}"
            );
            return Err(error.into());
        }
    };
    assert_eq!(settled, crate::gameplay_transport::SessionDeath::Durable);
    let receipt: (i64, i64, i64) = sqlx::query_as("SELECT experience_before,experience_after,experience_lost FROM game_character_death_receipts WHERE death_occurrence_id=encode($1,'hex')::uuid")
        .bind(death.occurrence.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    verify_xp(h, character, before_xp, actual_bosses, receipt.0).await?;
    assert_eq!(receipt.1, receipt.0 - receipt.2);
    let after: i64 = sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(after, receipt.1);
    assert_eq!(
        owner
            .settle_death(actor, destination.game_session_id, death, false)
            .await
            .0,
        crate::gameplay_transport::SessionDeath::Durable
    );
    driver
        .tick_soulpits(now + 5_000_000, true)
        .await
        .map_err(|_| "dead participant cadence")?;
    let mastery: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_animus_masteries WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        mastery, 0,
        "a dead participant acquires no completion mastery"
    );
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(receipts, 1);
    eprintln!(
        "SV8_SOULPIT_DEATH_ORDER_PG_EXECUTED bosses={actual_bosses} experience_before={} duration={:?}",
        receipt.0,
        began.elapsed()
    );
    Ok(())
}

pub(super) async fn verify_xp(
    h: &Harness,
    character: crate::domain::CharacterId,
    before: i64,
    bosses: i64,
    after: i64,
) -> TestResult<i64> {
    let (base, count): (i64, i64) = sqlx::query_as("SELECT coalesce(sum(base_xp),0)::bigint,count(*) FROM game_party_kill_xp_receipts WHERE principal_id=encode($1,'hex')::uuid AND creature_key='oteryn:creature.rat'")
        .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let (raw, awarded): (i64, i64) = sqlx::query_as("SELECT coalesce(sum(raw_share),0)::bigint,coalesce(sum(xp_awarded),0)::bigint FROM game_party_kill_xp_members WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(count, bosses);
    assert_eq!(base, 140 * bosses, "native rat Boss base5*28");
    assert_eq!(raw, base, "solo physical Boss owns the source share");
    assert_eq!(
        after,
        before + awarded,
        "actual hunting bonuses apply through the existing XP owner"
    );
    Ok(awarded)
}

/// Pin only the fixture's UUID through the real owner's test constructor. Every
/// position still comes from the production donor selector and actual room facts.
pub(super) async fn bind_identity(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    template: &QualifiedSoulpitRoomTemplate,
    scope: RuntimeScopeRefV1,
) -> TestResult {
    use crate::gameplay_transport::soulpit::{production::ProductionSoulpitHandoff, runtime};
    let assignment = owner
        .root
        .read_runtime_scope_assignment(scope)
        .await
        .map_err(debug)?
        .ok_or("ready Instance assignment")?;
    let generation =
        crate::foundation::ScopeOwnershipGeneration::new(assignment.ownership_generation)
            .map_err(debug)?;
    let root = owner
        .active_generation
        .ok_or("fixture active content")?
        .identity()
        .server_artifact_digest();
    let blocked = owner
        .door
        .lock()
        .await
        .blocking_cells()
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let cells = template.room().movement_cells();
    for nonce in 0..10_000_u32 {
        let mut handoff = id(221);
        handoff[..4].copy_from_slice(&nonce.to_be_bytes());
        let binding = runtime::SoulpitActivityBinding {
            handoff_id: handoff,
            scope,
            generation,
        };
        let mut collision_free = true;
        for wave in 1..=4 {
            let positions = runtime::source_wave_positions(root, binding, wave, |at| {
                runtime::source_spawn_tile(cells, &blocked, at)
            })
            .map_err(debug)?;
            let distinct = positions
                .iter()
                .map(|at| (at.x, at.y, at.floor))
                .collect::<std::collections::BTreeSet<_>>();
            if distinct.len() != positions.len()
                || positions.iter().any(|at| {
                    (at.x, at.y, at.floor) == (32373, 31151, 8)
                        || !runtime::source_spawn_tile(cells, &blocked, *at)
                })
            {
                collision_free = false;
                break;
            }
        }
        if collision_free {
            let production = ProductionSoulpitHandoff::with_test_handoff_id(handoff)
                .map_err(|_| "fixture UUID v7")?;
            owner
                .states
                .boss_activity_handoff
                .install(std::sync::Arc::new(production))
                .map_err(|_| "fixture Soulpit owner installed before first use")?;
            eprintln!("SV8_SOULPIT_FIXTURE_HANDOFF_NONCE {nonce}");
            return Ok(());
        }
    }
    Err("no collision-free fixture UUID under actual source selector".into())
}
