//! Fresh Slug corpses from an ordinary native kill, without seeding quest odours.

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_goblin_native_slug_death_materializes_fresh_corpse() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started=std::time::Instant::now();
    let result=super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-goblin-slug",||super::quest_world_callbacks_fixtures::scenario("qengine-rewards-goblin-slug",Position{x:33075,y:32109,floor:-7},false));
    eprintln!("QUESTPC7_GOBLIN_SLUG POSTGRES_CASE_COMPLETED duration={:?} success={}",started.elapsed(),result.is_ok());result
}
