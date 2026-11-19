//! Rafzan's chosen six-job start and exact source-priced tool bundle, without progress seeds.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;


#[test]
fn questpc7_goblin_six_jobs_and_exact_paid_tools() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-goblin-intro", || {
        super::quest_world_callbacks_fixtures::scenario(
            "qengine-rewards-goblin-intro",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_GOBLIN_INTRO POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn questpc7_goblin_three_personal_sign_sites() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-goblin-signs", || {
        super::quest_world_callbacks_fixtures::scenario("qengine-rewards-goblin-signs",
            Position { x: 33055, y: 32090, floor: -7 }, false)
    });
    eprintln!("QUESTPC7_GOBLIN_SIGNS POSTGRES_CASE_COMPLETED duration={:?} success={}", started.elapsed(), result.is_ok());
    result
}

#[test]
fn questpc7_goblin_exact_pond_consumes_rat_urine_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-goblin-pond", || {
        super::quest_world_callbacks_fixtures::scenario("qengine-rewards-goblin-pond",
            Position { x: 33134, y: 32092, floor: -10 }, false)
    });
    eprintln!("QUESTPC7_GOBLIN_POND POSTGRES_CASE_COMPLETED duration={:?} success={}", started.elapsed(), result.is_ok());
    result
}

#[test]
fn questpc7_goblin_three_trunks_retain_the_source_hammer() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started=std::time::Instant::now();
    let result=super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-goblin-trunks",||super::quest_world_callbacks_fixtures::scenario("qengine-rewards-goblin-trunks",Position{x:33101,y:32046,floor:-10},false));
    eprintln!("QUESTPC7_GOBLIN_TRUNKS POSTGRES_CASE_COMPLETED duration={:?} success={}",started.elapsed(),result.is_ok());
    result
}

#[test]
fn questpc7_goblin_five_native_stalkers_credit_exact_deaths() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started=std::time::Instant::now();
    let result=super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-goblin-stalkers",||super::quest_world_callbacks_fixtures::scenario("qengine-rewards-goblin-stalkers",Position{x:33075,y:32109,floor:-7},false));
    eprintln!("QUESTPC7_GOBLIN_STALKERS POSTGRES_CASE_COMPLETED duration={:?} success={}",started.elapsed(),result.is_ok());result
}

#[test]
fn questpc7_goblin_perfume_source_items_admit_without_free_grants() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started=std::time::Instant::now();
    let result=super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-goblin-perfume-items",||super::quest_world_callbacks_fixtures::scenario("qengine-rewards-goblin-perfume-items",Position{x:33294,y:31677,floor:-15},false));
    eprintln!("QUESTPC7_GOBLIN_PERFUME_ITEMS POSTGRES_CASE_COMPLETED duration={:?} success={}",started.elapsed(),result.is_ok());result
}
