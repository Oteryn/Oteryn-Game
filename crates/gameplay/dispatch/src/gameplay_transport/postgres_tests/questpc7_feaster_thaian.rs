//! First-party Thaian data admission after the real Feaster introduction; no boss-clear claim.

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_feaster_thaian_native_admission_preserves_source_facts() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none(){return Ok(());}
    let start=std::time::Instant::now();
    let result=super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-thaian",||super::quest_world_callbacks_fixtures::scenario("qengine-rewards-feaster-thaian",Position{x:32627,y:32076,floor:-7},false));
    eprintln!("QUESTPC7_THAIAN POSTGRES_CASE_COMPLETED duration={:?} success={}",start.elapsed(),result.is_ok());result
}
