//! Lane qc: Obsidian Knife and Sinatuki through native NPC wire commands.
use super::TestResult;
use super::qc_npc_programmes_fixtures::START;

#[test]
fn qc_obsidian_knife_friend_cyclops_forges_draconian_steel_repeatably() -> TestResult {
    run("qc-obsidian-knife")
}

#[test]
fn qc_sinatuki_ninety_nine_then_one_fish_grants_shards_and_jamjam() -> TestResult {
    run("qc-sinatuki")
}

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, START, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
