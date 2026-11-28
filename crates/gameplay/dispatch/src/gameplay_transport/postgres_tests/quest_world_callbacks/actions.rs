use super::*;

#[test]
fn quest_world_callbacks_qcb3_real_dispatch_actions() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("quest-qcb3-actions", || {
        for name in [
            "outlaw-camp-door",
            "travelling-trader-delivery",
            "fish-napping-key",
        ] {
            scenario(
                name,
                Position {
                    x: 32358,
                    y: 32788,
                    floor: -6,
                },
                false,
            )
            .map_err(|error| format!("{name}: {error}"))?;
        }
        Ok(())
    })
}

#[test]
fn quest_world_callbacks_qcb3_target_swap_with_consumption_and_teleport() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED qcb3_consuming_tool_regression");
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("quest-qcb3-target-swap", || {
        scenario(
            "fish-napping-key-target-swap",
            Position {
                x: 32358,
                y: 32788,
                floor: -6,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcb3_consuming_tool_regression duration={:?}",
        started.elapsed()
    );
    Ok(())
}
