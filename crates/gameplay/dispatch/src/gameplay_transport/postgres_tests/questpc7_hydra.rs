//! Native Hydra entry, guardian evasion and personal ordinary-map egg custody.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_hydra_requires_personal_native_nest_pickup() -> TestResult {
 if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none(){return Ok(());}
 let started=std::time::Instant::now();
 let result=super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-hydra",||super::quest_world_callbacks_fixtures::scenario("questpc7-hydra",Position{x:33003,y:32665,floor:-3},false));
 eprintln!("QUESTPC7_HYDRA POSTGRES_CASE_COMPLETED duration={:?} success={}",started.elapsed(),result.is_ok());result
}
