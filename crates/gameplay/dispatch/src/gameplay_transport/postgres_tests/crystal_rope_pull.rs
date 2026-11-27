use super::TestResult;
use super::vertical_move_fixtures::{RopeUse, agent_rope};

#[test]
fn crystal_rope_tools_pull_ground_stack_through_open_hole() -> TestResult {
    exercise(RopeUse::Pull, &[3003, 646, 9594, 9596, 9598])
}

#[test]
fn crystal_rope_pulls_creature_before_ground_stack() -> TestResult {
    exercise(RopeUse::PullCreature, &[3003])
}

#[test]
fn crystal_rope_skips_occupied_creature_landing() -> TestResult {
    exercise(RopeUse::PullCreatureFromSouth, &[3003])
}

#[test]
fn crystal_rope_pulled_player_gets_stairhop_cooldown() -> TestResult {
    exercise(RopeUse::PullPlayer, &[3003])
}

#[test]
fn crystal_rope_skips_full_item_landing() -> TestResult {
    exercise(RopeUse::PullFromFullLanding, &[3003])
}

fn exercise(action: RopeUse, tools: &'static [u32]) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            super::fixture::run(async |admin| {
                for id in tools {
                    let key = format!("oteryn:item.tibia.i{id}");
                    agent_rope(admin.clone(), &key, 385, action).await?;
                }
                eprintln!("POSTGRES_CASE_COMPLETED rope pull for {tools:?}");
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Crystal rope pull dispatch panicked")?
        .map_err(Into::into)
}
