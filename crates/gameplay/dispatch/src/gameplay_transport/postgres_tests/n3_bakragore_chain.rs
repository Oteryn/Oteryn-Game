use super::TestResult;
use super::srx3_rotten_advanced_fixtures::{Extra, scenario_with_step};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::MovementLocalPosition;

#[test]
fn n3_bakragore_source_lever_quest_private_reward_and_return_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    scenario_with_step(
        PinnedActivitySourceRoom::Bakragore,
        MovementLocalPosition {
            x: 33010,
            y: 32392,
            floor: 15,
        },
        "oteryn:creature.bakragore",
        true,
        false,
        Extra::BakragoreSettlement,
    )?;
    eprintln!(
        "N3_BAKRAGORE_CHAIN POSTGRES_CASE_EXECUTED duration={:?}",
        started.elapsed()
    );
    Ok(())
}
