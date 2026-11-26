use super::super::fixture::{Harness, debug, id};
use super::super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::super::{TestResult, prepare_current_session};
use super::{AttackCase, dependencies_at, health, initialize_player};
use oteryn_gameplay_entities::carrier::runtime_actor_reentry::FRESH_LOGIN_PROTECTION_MICROS;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::time::Instant;

#[derive(Clone, Copy)]
enum Scenario {
    Adjacent,
    OutOfRange,
    LoginProtected,
    ProtectionZone,
    RangedProfile,
}

#[test]
fn due_rat_melee_applies_damage_while_profile_checkout_is_refused() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run("mmelee_adjacent", &[Scenario::Adjacent])
}

#[test]
fn refused_profile_checkout_keeps_melee_target_checks() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(
        "mmelee_target_checks",
        &[
            Scenario::OutOfRange,
            Scenario::LoginProtected,
            Scenario::ProtectionZone,
        ],
    )
}

#[test]
fn refused_monster_profile_checkout_retries_the_due_profile() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run("mmelee_profile_retry", &[Scenario::RangedProfile])
}

fn run(tag: &'static str, scenarios: &'static [Scenario]) -> TestResult {
    run_async(move |admin| async move {
        let harness = Harness::create_with_admission_seed(admin, tag, false, false).await?;
        let result = exercise(&harness, scenarios).await;
        let cleanup = harness.cleanup().await;
        result?;
        cleanup
    })
}

async fn exercise(harness: &Harness, scenarios: &[Scenario]) -> TestResult {
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
    let active = controller.active().ok_or("active native generation")?;
    let cadence = crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS * 1_000;

    for scenario in scenarios {
        let (case, player_at, monster_at) = match scenario {
            Scenario::Adjacent | Scenario::LoginProtected => {
                (AttackCase::Rat, (3, 12, -7), (2, 12, -7))
            }
            Scenario::OutOfRange => (AttackCase::Rat, (3, 12, -7), (0, 12, -7)),
            Scenario::ProtectionZone => (AttackCase::Rat, (-2, 10, -7), (-2, 11, -7)),
            Scenario::RangedProfile => (AttackCase::Hunter, (3, 12, -7), (-1, 12, -7)),
        };
        let login_protected = matches!(scenario, Scenario::LoginProtected);
        let (deps, monster) = dependencies_at(active, &pin, case, player_at, monster_at, Some(0))?;
        let mut owner = deps.owner(harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = deps.room.movement_cells().into();
        let max_health = crate::gameplay_transport::actor_spell::tests::FACTS.max_health;
        initialize_player(
            &owner,
            deps.actor,
            deps.admitted.game_session_id,
            max_health,
        )
        .await?;

        let tiles = deps.room.movement_cells();
        let is_pz = |(x, y, z): (i32, i32, i16)| -> TestResult<bool> {
            Ok(tiles
                .spell_tiles()
                .lookup(
                    tiles.scope(),
                    crate::content::LogicalCell {
                        x,
                        y,
                        z: i32::from(z),
                    },
                )
                .map_err(debug)?
                .flags()
                .protection_zone)
        };
        assert!(!is_pz(monster_at)?);
        assert_eq!(
            is_pz(player_at)?,
            matches!(scenario, Scenario::ProtectionZone)
        );
        assert_eq!(
            player_at
                .0
                .abs_diff(monster_at.0)
                .max(player_at.1.abs_diff(monster_at.1))
                > 1,
            matches!(scenario, Scenario::OutOfRange | Scenario::RangedProfile)
        );

        let first = if login_protected {
            0
        } else {
            FRESH_LOGIN_PROTECTION_MICROS + 1
        };
        let before = health(&owner, &deps).await?;
        harness.root.request_ready();
        assert!(harness.root.maintain_ready_once().await?);
        let mut holder = harness.root.try_acquire_ready()?;
        // Holding the checkout is the readiness barrier for the entire busy phase.
        assert!(matches!(
            harness.root.try_issue_background_semantic_pass(),
            Err(crate::durability::DurabilityError::RootUnavailable)
        ));
        let mut now = first;
        // The canonical Rat may roll zero. Drive whole source think intervals, while
        // keeping all login-protected attempts inside the real protection window.
        for _ in 0..if login_protected {
            FRESH_LOGIN_PROTECTION_MICROS / cadence
        } else {
            16
        } {
            owner
                .drain_monster_melee_at(SemanticTimeMicros::from_micros(now))
                .await;
            if matches!(scenario, Scenario::Adjacent) && health(&owner, &deps).await? < before {
                break;
            }
            now += cadence;
        }

        let after = health(&owner, &deps).await?;
        if matches!(scenario, Scenario::Adjacent) {
            assert!(
                after < before,
                "due source melee must apply damage before the holder returns"
            );
        } else {
            assert_eq!(
                after, before,
                "a refused checkout must not bypass target eligibility"
            );
        }

        if matches!(scenario, Scenario::Adjacent) {
            owner
                .drain_monster_melee_at(SemanticTimeMicros::from_micros(now))
                .await;
            assert_eq!(
                health(&owner, &deps).await?,
                after,
                "the same due turn cannot hit twice"
            );
        }

        let selection = owner
            .states
            .spell_states
            .lock()
            .await
            .monster_melee
            .creature_ai
            .native_melee_selection(monster, 0)
            .ok_or("admitted monster")?;
        if matches!(scenario, Scenario::RangedProfile) {
            assert_eq!(
                selection.occurrence.sequence, 0,
                "profile still awaits its checkout"
            );
        } else {
            assert!(
                selection.occurrence.sequence > 0,
                "the independent melee turn ran"
            );
        }
        let _ = holder
            .return_to_pool_observed_until(Instant::now() + crate::durability::DB_PASS_DEADLINE)
            .await;
        assert!(harness.root.is_ready());

        if matches!(scenario, Scenario::RangedProfile) {
            // The original due think was retained. Re-enter at the same owner time
            // and check that the profile publishes its first real think and deadline.
            owner
                .drain_monster_melee_at(SemanticTimeMicros::from_micros(first + 15 * cadence))
                .await;
            {
                let states = owner.states.spell_states.lock().await;
                let ai = &states.monster_melee.creature_ai;
                assert_eq!(
                    ai.state(monster).and_then(|state| state.target()),
                    Some(deps.actor)
                );
                assert_eq!(
                    ai.native_melee_selection(monster, 0)
                        .ok_or("profile turn")?
                        .occurrence
                        .sequence,
                    1
                );
                assert!(ai.due(monster).is_some_and(|due| due >= now));
            }
            // Prove the delayed owner also publishes profile effects. The Hunter
            // starts beyond melee range and its ranged attack can roll zero.
            for _ in 0..32 {
                super::assert_distant(&owner, monster, deps.actor).await?;
                owner
                    .drain_monster_melee_at(SemanticTimeMicros::from_micros(now))
                    .await;
                if health(&owner, &deps).await? < before {
                    break;
                }
                now += cadence;
            }
            assert!(
                health(&owner, &deps).await? < before,
                "the retried ranged profile must apply damage"
            );
        }
    }
    Ok(())
}
