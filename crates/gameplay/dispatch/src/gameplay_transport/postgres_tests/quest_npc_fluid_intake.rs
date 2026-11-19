use super::TestResult;
use super::quest_npc_fluid_intake_fixtures::run;

#[test]
fn quest_npc_rapanaio_consumes_only_rum_and_fruit_juice_at_their_topics() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run("quest-npc-exact-fluid-rapanaio")
}

#[test]
fn quest_npc_lazaran_exchanges_only_beer_for_the_tribal_crest() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run("quest-npc-exact-fluid-lazaran")
}
