//! Native objective guardians and exact lethal credit, without quest progress fixtures.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_native_objectives_spawn_once_and_credit_exact_lethal_occurrences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let start=std::time::Instant::now();
    let result=super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-native-deaths",|| {
        super::quest_world_callbacks_fixtures::scenario("questpc7-hidden-deaths",Position {x:33079,y:32015,floor:-13},false)
    });
    eprintln!("QUESTPC7_NATIVE_DEATHS POSTGRES_CASE_COMPLETED duration={:?} success={}",start.elapsed(),result.is_ok());
    result
}

#[test]
fn questpc7_hidden_full_native_route_and_atomic_valuables() -> TestResult {
 if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none(){return Ok(());}
 let start=std::time::Instant::now();
 let result=super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-hidden-story",||super::quest_world_callbacks_fixtures::scenario("questpc7-hidden-story",Position{x:33079,y:32015,floor:-13},false));
 eprintln!("QUESTPC7_HIDDEN_STORY POSTGRES_CASE_COMPLETED duration={:?} success={}",start.elapsed(),result.is_ok()); result
}
