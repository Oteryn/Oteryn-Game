use super::party_xp_fixtures::{Case, run_with_admin};
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn pcake_kill_before_activation_drains_in_window_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_with_admin(Case::CakeKillBeforeActivation, admin)
}

#[test]
fn pcake_kill_in_window_drains_after_expiry_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_with_admin(Case::CakeKillBeforeExpiry, admin)
}

#[test]
fn pcake_regeneration_ticks_inside_house_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::hireling_lifecycle_fixtures::run_case_with_admin(
        super::hireling_lifecycle_fixtures::Case::CakeHouseRegeneration,
        admin,
    )
}

#[test]
fn pcake_world_bonus_expired_real_creature_death_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_with_admin(Case::CakeWorldBonusExpired, admin)
}

#[test]
fn pcake_world_bonus_real_creature_death_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_with_admin(Case::CakeWorldBonus, admin)
}

#[test]
fn pcake_regeneration_failed_poll_is_backed_off_postgres() -> TestResult {
    use super::Dependencies;
    use super::fixture::{Harness, debug, run};

    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("cake-poll-backoff", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "cake_poll_backoff", true, false)
                .await?;
            {
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let deps = Dependencies::new()?;
                let owner = deps.owner(&h, &authority);
                sqlx::query("ALTER TABLE game_cake_phases RENAME TO test_unavailable_cake_phases")
                    .execute(&h.pool)
                    .await?;
                assert!(matches!(
                    owner.refresh_cake_regeneration().await,
                    Err(crate::durability::DurabilityError::Database(_))
                ));
                assert!(
                    owner.refresh_cake_regeneration().await.is_ok(),
                    "the failed query starts the same backoff as a successful poll"
                );
            }
            h.cleanup().await
        })
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED cake_poll_backoff elapsed={:?}",
        started.elapsed()
    );
    Ok(())
}
