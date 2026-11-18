//! LOOP-MONSTER-1 through the real Channel dispatcher and configured PostgreSQL 17.6.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
#[path = "loop_monster/fixture.rs"]
pub(super) mod fixture;
use fixture::{cell, dependencies, next_pass, run_case};
#[path = "loop_monster/item_scope.rs"]
mod item_scope;
#[path = "loop_monster/pass_cost.rs"]
mod pass_cost;
#[path = "loop_monster/shared_budget.rs"]
mod shared_budget;
#[path = "loop_monster/sql_cancellation.rs"]
mod sql_cancellation;
#[path = "loop_monster/summon_transaction.rs"]
mod summon_transaction;
#[path = "loop_monster/unknown_bundle_field.rs"]
mod unknown_bundle_field;

#[test]
fn current_owner_spawn_home_walkback_and_leash_use_actual_source_cell() -> TestResult {
    run_case(async |harness| {
        prepare_current_session(harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let active = controller.active().ok_or("active generation")?;
        let (deps, rat) = dependencies(active, &pin)?;
        let mut owner = deps.owner(harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (deps.room.movement_cells()).into();
        owner.clock_origin -= std::time::Duration::from_secs(10);
        // No PlayerSpellState is installed: the actual think has no eligible target.
        owner.drain_monster_melee().await;
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(rat)
                .map_err(debug)?
                .position(),
            cell(2)
        );
        sql_cancellation::cancel_then_retry(harness, &deps, &mut owner, rat).await?;
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(rat)
                .map_err(debug)?
                .position(),
            cell(3)
        );
        next_pass(&mut owner).await?;
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(rat)
                .map_err(debug)?
                .position(),
            cell(4)
        );
        assert_eq!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .monster_melee
                .creature_ai
                .due(rat),
            None
        );
        // Inject a valid exact-actor displacement beyond the leash radius. The real current
        // Map/Item pass must resolve the authored home rather than the admission cell.
        {
            let mut runtime = deps.runtime.lock().await;
            let before = runtime.read_actor_position(rat).map_err(debug)?;
            runtime
                .borrow_movement_position()
                .commit_creature_relocation(rat, before, cell(-47))
                .map_err(debug)?;
        }
        owner
            .states
            .spell_states
            .lock()
            .await
            .monster_melee
            .creature_ai
            .wake(cell(-47), |_| Some(cell(-47)), owner.owner_now().get());
        owner.clock_origin -= std::time::Duration::from_secs(1);
        owner.drain_monster_melee().await;
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(rat)
                .map_err(debug)?
                .position(),
            cell(4)
        );
        // Occupied home refuses without moving its occupant or the leashed creature.
        {
            let mut runtime = deps.runtime.lock().await;
            let before = runtime.read_actor_position(rat).map_err(debug)?;
            runtime
                .borrow_movement_position()
                .commit_creature_relocation(rat, before, cell(-47))
                .map_err(debug)?;
            for at in [
                MovementLocalPosition {
                    x: 3,
                    y: 11,
                    floor: -7,
                },
                MovementLocalPosition {
                    x: 4,
                    y: 11,
                    floor: -7,
                },
                cell(4),
            ] {
                let before = runtime.read_actor_position(deps.actor).map_err(debug)?;
                runtime
                    .borrow_movement_position()
                    .commit_cardinal(before, at)
                    .map_err(debug)?;
            }
        }
        owner
            .states
            .spell_states
            .lock()
            .await
            .monster_melee
            .creature_ai
            .wake(cell(-47), |_| Some(cell(-47)), owner.owner_now().get());
        owner.clock_origin -= std::time::Duration::from_secs(1);
        owner.drain_monster_melee().await;
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(rat)
                .map_err(debug)?
                .position(),
            cell(-47)
        );
        Ok(())
    })
}

#[test]
fn current_owner_summon_transaction_map_and_one_pass_use_actual_item_scope() -> TestResult {
    run_case(async |harness| summon_transaction::scenario(harness, false, false).await)
}
#[test]
fn activated_outer_source_summon_rejects_inner_catalog_and_registers_once() -> TestResult {
    run_case(async |harness| summon_transaction::scenario(harness, true, false).await)
}

#[test]
fn current_owner_summon_registration_failure_rolls_back_and_replays_the_refusal() -> TestResult {
    run_case(async |harness| summon_transaction::scenario(harness, false, true).await)
}

#[test]
fn resumed_summon_uses_current_path_budget_window() -> TestResult {
    run_case(async |harness| summon_transaction::resumed_budget(harness).await)
}

#[test]
fn checkout_paths_keep_one_window_across_sql_wait() -> TestResult {
    run_case(async |harness| summon_transaction::checkout_window(harness).await)
}

#[test]
fn exhausted_checkout_retains_the_entire_think_until_next_window() -> TestResult {
    run_case(async |harness| summon_transaction::exhausted_window(harness).await)
}

#[test]
fn review67_composed_summon_preserves_the_queued_player_reserve() -> TestResult {
    run_case(async |harness| summon_transaction::queued_player(harness).await)
}

#[test]
fn review67_sql_watchdog_expires_during_replay_sql() -> TestResult {
    run_case(async |harness| summon_transaction::blocked_replay(harness).await)
}

#[test]
fn route_less_library_gate_omits_the_postgres_body() -> TestResult {
    const PROBE: &str = "OTERYN_LOOP_MONSTER_ROUTELESS_PROBE";
    if std::env::var_os(PROBE).is_some() {
        return run_case(async |_| Err("route-less gate executed a database body".into()));
    }
    let output = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", "gameplay_transport::monk_save::postgres_tests::loop_monster::route_less_library_gate_omits_the_postgres_body", "--nocapture"])
        .env_remove("OTERYN_TEST_POSTGRES_ADMIN_URL").env(PROBE, "1").output()?;
    assert!(output.status.success(), "the required route-less library gate must succeed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Loop Monster database body omitted; no PostgreSQL validation claimed"));
    assert!(!stderr.contains("LOOP-MONSTER-PG: configured PostgreSQL"));
    Ok(())
}

#[test]
fn consecutive_exhausted_windows_replace_the_expired_retry_deadline() -> TestResult {
    run_case(async |harness| summon_transaction::consecutive_windows(harness).await)
}

#[test]
fn initial_checkout_timeout_is_not_a_replay_watchdog_proof() -> TestResult {
    run_case(async |harness| summon_transaction::blocked_initial_checkout(harness).await)
}
