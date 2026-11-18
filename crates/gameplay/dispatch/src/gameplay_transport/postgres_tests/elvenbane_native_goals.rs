//! Elvenbane's personal defender goals use source-native death occurrences.
use super::TestResult;
use super::fixture::{Harness, run};

#[test]
fn elvenbane_source_defender_goal_catalogue_is_closed() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "elvenbane_native_goals", false, false)
            .await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&h.pool)
            .await?;
        println!("ELVENBANE_NATIVE_GOALS POSTGRES_CASE_OBSERVED server_version_num={version}");
        let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        assert!(
            lowered
                .catalogue()
                .track("oteryn:quest-progress/elvenbane/native-defenders/dragon")
                .is_some(),
            "Elvenbane requires a bounded native defender goal, not a client-written completion flag"
        );
        h.cleanup().await?;
        Ok(())
    })?;
    println!(
        "ELVENBANE_NATIVE_GOALS POSTGRES_CASE_COMPLETED duration_seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

#[test]
fn elvenbane_native_defenders_capture_refuse_replay_and_cap_per_origin() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("elvenbane-native-goals", || {
        super::quest_world_callbacks_fixtures::scenario(
            "elvenbane-native-goals",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 32598,
                y: 31641,
                floor: -8,
            },
            false,
        )
    })?;
    println!(
        "ELVENBANE_NATIVE_GOALS POSTGRES_CASE_COMPLETED duration_seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
