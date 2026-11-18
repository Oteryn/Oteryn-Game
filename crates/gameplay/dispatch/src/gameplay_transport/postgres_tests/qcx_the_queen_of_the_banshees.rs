use super::{TestResult, kill_reward_e2e::scenario_thread, quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::{self, Position};

#[test]
fn final_room_forcefield_returns_to_ghostlands_surface() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }

    scenario_thread::run("qcx76-final-room-exit", || {
        let started = std::time::Instant::now();
        let name = "banshees-final-battle-exit";
        quest_world_callbacks::scenario(
            name,
            Position {
                x: 32219,
                y: 31913,
                floor: -15,
            },
            false,
        )?;
        eprintln!("qcx76 final room dispatch {:.2?}", started.elapsed());

        // The scenario checks the dispatched position against the loaded callback
        // and verifies its durable seal resets. Compare that same callback with
        // the independent Mapper Coords in /Spoiler, curid 34100, oldid 1057212.
        let callback = world_callbacks::load()?
            .into_iter()
            .find(|callback| callback.key.ends_with(name))
            .ok_or("final battle exit callback missing")?;
        assert_eq!(
            callback.teleport,
            Some(Position {
                x: 125 * 256 + 202,
                y: 124 * 256 + 102,
                floor: -7,
            })
        );
        Ok(())
    })
}
