//! Reuse the registered Bestiary/revision harness and the existing composed Dependencies.
use super::*;
use crate::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

pub(super) fn run_case(
    body: impl AsyncFnOnce(&Harness) -> TestResult + Send + 'static,
) -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        // The required lib-rest gate is intentionally route-less. Only the
        // separately routed PostgreSQL lane can establish database evidence.
        eprintln!("PRE-ROUTING / NONCANONICAL: Loop Monster database body omitted; no PostgreSQL validation claimed");
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            let rt = crate::bestiary_postgres_harness::runtime()
                .map_err(|error| format!("{error:?}"))?;
            let harness = rt.block_on(async {
                tokio::time::timeout(std::time::Duration::from_secs(30),
                    Harness::create_with_admission_seed(admin, "loop_monster", false, false))
                    .await.map_err(|_| "Loop Monster setup watchdog expired")?
            }).map_err(|error| format!("{error:?}"))?;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                rt.block_on(async {
                    tokio::time::timeout(std::time::Duration::from_secs(30), async {
                    let version: String = sqlx::query_scalar("SHOW server_version_num")
                        .fetch_one(&harness.pool)
                        .await?;
                    assert_eq!(
                        version,
                        oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM,
                        "the accepted PostgreSQL version"
                    );
                    eprintln!(
                        "LOOP-MONSTER-PG: configured PostgreSQL server_version_num={version}"
                    );
                    body(&harness).await
                    }).await.map_err(|_| "Loop Monster case watchdog expired")?
                })
            }));
            match result {
                Ok(result) => rt.block_on(finish_case(result, harness.cleanup())).map_err(|error| format!("{error:?}")),
                Err(panic) => {
                    let _cleanup = rt.block_on(finish_case(Ok(()), harness.cleanup()));
                    std::panic::resume_unwind(panic)
                }
            }
        })?
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
        .map_err(Into::into)
}

async fn finish_case(result: TestResult, cleanup: impl std::future::Future<Output = TestResult>) -> TestResult {
    let cleanup = tokio::time::timeout(std::time::Duration::from_secs(1), cleanup).await
        .map_err(|_| "Loop Monster cleanup watchdog expired".into()).and_then(|result| result);
    match result {
        Err(error) => {
            if let Err(cleanup) = cleanup {
                eprintln!("LOOP-MONSTER-PG cleanup failed after case failure: {cleanup}");
            }
            Err(error)
        }
        Ok(()) => cleanup,
    }
}

#[test]
fn review67_cleanup_watchdog_preserves_the_body_error() -> TestResult {
    crate::bestiary_postgres_harness::runtime()?.block_on(async {
        let outcome = tokio::time::timeout(std::time::Duration::from_millis(1500),
            finish_case(Err("original SQL-wait watchdog expired".into()), async {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                Ok(())
            })).await;
        let result = outcome.map_err(|_| "cleanup hid the scenario timeout")?;
        assert_eq!(result.err().ok_or("lost body timeout")?.to_string(), "original SQL-wait watchdog expired");
        Ok(())
    })
}

pub(super) fn cell(x: i32) -> MovementLocalPosition {
    MovementLocalPosition {
        x,
        y: 12,
        floor: -7,
    }
}

pub(in super::super::super) fn dependencies(
    active: &crate::content::ActiveGeneration,
    pin: &crate::content::NativeEntryContentPin,
) -> TestResult<(Dependencies, ExactActorRef)> {
    dependencies_with_capacity(active, pin, 8)
}

pub(super) fn dependencies_with_capacity(
    active: &crate::content::ActiveGeneration,
    pin: &crate::content::NativeEntryContentPin,
    capacity: usize,
) -> TestResult<(Dependencies, ExactActorRef)> {
    let mut deps = Dependencies::new()?;
    deps.room = pin.qualified_room().clone();
    deps.combat_room = Some(deps.room.clone());
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let channel_pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        pin.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        deps.room.map_revision_digest(),
        (2, 11, -7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        capacity,
        channel_pin,
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    // This production World census owner supplies the authored home, independently of the
    // later displaced admission position used by the scenario.
    use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};
    let census = Table {
        creatures: vec![Creature {
            key: "oteryn:creature.rat".into(),
            period: Period::All,
        }],
        sources: vec![Source {
            key: "oteryn:spawn/loop-monster-home".into(),
            floor: 7,
            x: 4,
            y: 12,
            points: vec![Point {
                creature: 0,
                x: 4,
                y: 12,
                direction: Direction::West,
                respawn_ms: 60_000,
                boss_policy: None,
            }],
        }],
    };
    let facts = crate::map::boot::table_activation_spawns(&census, |key| {
        let policy = runtime.companion_policy(key).ok()?;
        Some((u64::try_from(policy.maximum_health).ok()?, false))
    })
    .ok_or("source spawn policies")?;
    runtime
        .begin_activation_spawn_sources(facts)
        .map_err(debug)?;
    assert_eq!(
        runtime
            .realize_activation_spawn_window(0)
            .map_err(debug)?
            .realized,
        1
    );
    let rat = runtime
        .visible_entities()
        .creatures
        .first()
        .ok_or("spawned rat")?
        .actor;
    assert_eq!(runtime.creature_spawn_cell(rat), Some(cell(4)));
    crate::movement::relocate::relocate_creature(
        &mut runtime,
        rat,
        deps.room.movement_cells(),
        &std::collections::BTreeSet::new(),
        cell(2),
    )
    .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    Ok((deps, rat))
}

pub(super) async fn next_pass(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let rat = runtime.spawn_point_creatures()[0].2.ok_or("spawned rat")?;
    let due = states
        .monster_melee
        .creature_ai
        .step_due_for_test(rat)
        .unwrap_or(states.next_monster_ai_pass_us);
    drop(runtime);
    drop(states);
    owner.clock_origin -=
        std::time::Duration::from_micros(due.saturating_sub(owner.owner_now().get()) + 100);
    owner.drain_monster_melee().await;
    Ok(())
}
