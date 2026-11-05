//! Source position USE selects the real above-ground object, not the tile's ground.
use super::{TestResult, quest_world_callbacks::stateless_scenario};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) fn ordinal(key: &str) -> Option<u8> {
    match key.rsplit('/').next()? {
        "blue-valley-2" | "blue-valley-3" => Some(3),
        "blue-valley-4" | "pirates-tail-cheesy-exit" | "order-of-the-stag-crypt-exit" => Some(1),
        "blue-valley-5" | "order-of-the-stag-boat-return" => Some(2),
        _ => None,
    }
}

#[test]
fn quest_position_callbacks_literal_use_matches_all_seven_source_objects() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    super::kill_reward_e2e::scenario_thread::run("quest-position-use", || {
        for (name, from, to) in [
            ("blue-valley-2", (33605,31468,-7), (32456,32291,-7)),
            ("blue-valley-3", (33601,31468,-7), (32456,32291,-7)),
            ("blue-valley-4", (32456,32290,-7), (33614,31494,-7)),
            ("blue-valley-5", (32203,32303,-6), (33614,31494,-7)),
            ("pirates-tail-cheesy-exit", (33925,31355,-7), (33799,31321,-7)),
            ("order-of-the-stag-boat-return", (33874,32250,-7), (32395,32511,-7)),
            ("order-of-the-stag-crypt-exit", (32402,32472,-5), (32402,32473,-4)),
        ] {
            let p = |(x,y,floor)| Position { x,y,floor };
            let started = std::time::Instant::now();
            stateless_scenario(name,p(from),p(to))?;
            eprintln!("QUEST_POSITION_USE_POSTGRES_CASE_COMPLETED {name} {:?}", started.elapsed());
        }
        Ok(())
    })
}
