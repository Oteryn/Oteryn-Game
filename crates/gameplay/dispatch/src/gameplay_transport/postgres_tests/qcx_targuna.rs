//! TibiaWiki Targuna Quest/Spoiler (curid 110238, oldid 1208547).
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn sterling_opens_wiki_turn_in_and_records_one_marker() -> TestResult {
    timed("sterling", || {
        super::m16_personal_map_marker::m16_sterling_commits_personal_marker_and_delivers_delta_and_reconnect_snapshot()
    })
}

#[test]
fn hidden_lizard_temple_forcefield_returns_to_targuna() -> TestResult {
    return_trip(
        "temple-return",
        "oteryn:quest-callback/targuna-temple-return",
        Position {
            x: 31941,
            y: 31928,
            floor: -9,
        },
        Position {
            x: 31935,
            y: 31861,
            floor: -7,
        },
    )
}

#[test]
fn matilda_returns_from_aragonia_to_targuna() -> TestResult {
    return_trip(
        "matilda-return",
        "oteryn:quest-callback/targuna-matilda-return",
        Position {
            x: 33478,
            y: 32729,
            floor: -7,
        },
        Position {
            x: 31925,
            y: 31907,
            floor: -7,
        },
    )
}

fn return_trip(
    name: &'static str,
    key: &'static str,
    trigger: Position,
    destination: Position,
) -> TestResult {
    timed(name, || {
        super::kill_reward_e2e::scenario_thread::run(name, move || {
            super::quest_world_callbacks::stateless_scenario(key, trigger, destination)
        })
    })
}

fn timed(name: &str, run: impl FnOnce() -> TestResult) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    let result = run();
    eprintln!("QCX73 {name} duration={:?}", start.elapsed());
    result
}
