//! Lane qc: Sanctuary of the Lizard God egg altar through the production world-callback owner.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn qc_lizard_sanctuary_first_altar_half_takes_the_eggs_once() -> TestResult {
    run("qc-lizard-sanctuary-altar-1")
}

#[test]
fn qc_lizard_sanctuary_second_altar_half_takes_the_eggs_once() -> TestResult {
    run("qc-lizard-sanctuary-altar-2")
}

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let trigger: Position = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == format!("oteryn:quest-callback/{name}"))
        .and_then(|c| c.selector.position)
        .ok_or("qc lizard sanctuary callback")?;
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, trigger, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
