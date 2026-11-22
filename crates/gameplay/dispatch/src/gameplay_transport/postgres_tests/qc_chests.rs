//! Lane qc: personal quest chests through the production world-callback owner.
use super::TestResult;

#[test]
fn qc_rapier_chest_grants_one_rapier_completes_the_quest_and_refuses_a_second_claim() -> TestResult
{
    run("qc-rapier-chest")
}

/// Skull of Ratha: both bags in the tomb and the underground chest (TibiaWiki
/// Skull of Ratha Quest), at every source placement through the catalogue owner.
#[test]
fn qc_skull_of_ratha_three_source_chests_use_replay_and_refusal() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    const CLAIMS: [&str; 3] = [
        "oteryn:reward-claim.kv.witch_house_quest",
        "oteryn:reward-claim.quest.u7_6.explorer_society.skull_of_ratha.bag1",
        "oteryn:reward-claim.quest.u7_6.explorer_society.skull_of_ratha.bag2",
    ];
    let selected: Vec<_> = super::quest_catalogue_chests_fixtures::cases()?
        .into_iter()
        .filter(|case| CLAIMS.contains(&case.claim.as_str()))
        .collect();
    assert_eq!(
        selected
            .iter()
            .map(|case| case.claim.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        CLAIMS.into_iter().collect(),
        "every Skull of Ratha reward must execute"
    );
    let started = std::time::Instant::now();
    super::quest_catalogue_chests_fixtures::execute(selected)?;
    super::m35_claim_sets::skull_of_ratha()?;
    println!(
        "POSTGRES_CASE_COMPLETED qc_skull_of_ratha_three_source_chests_use_replay_and_refusal duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn qc_sanctuary_pinned_legion_helmet_chest_uses_live_uid() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let cases: Vec<_> = super::quest_catalogue_chests_fixtures::cases()?
        .into_iter()
        .filter(|case| case.claim == "oteryn:reward-claim.kv.legiondawnport")
        .collect();
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].unique, 6006);
    super::quest_catalogue_chests_fixtures::execute(cases)
}

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == format!("oteryn:quest-callback/{name}"))
        .ok_or("qc chest callback")?;
    let trigger = callback.selector.position.ok_or("qc chest position")?;
    let started = std::time::Instant::now();
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
