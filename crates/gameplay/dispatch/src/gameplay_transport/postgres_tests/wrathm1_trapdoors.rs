use super::{ComposedFreshAdmission, TestResult};
use crate::map::{overlay::TilePos, view::compose_tile};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) const NAME: &str = "qengine-rewards-wrathm1-trapdoor-clock";
pub(super) const START: Position = super::wrathm1_prison_release::START;
const DOOR: TilePos = TilePos { x: 33385, y: 31139, floor: -8 };

#[test]
fn wrathm1_trapdoor_channel_clock_creates_and_cycles_the_source_door() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, START, false)
    })
}

pub(super) fn world(native: &crate::content::native_gameplay::NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    super::questpc8_native_slice::world(
        native, &[(START.x, START.y, START.x, START.y, START.floor), (DOOR.x, DOOR.y, DOOR.x, DOOR.y, DOOR.floor)],
        &[11257, 11258], TilePos { x: START.x, y: START.y, floor: START.floor },
    )
}

pub(super) async fn exercise(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    let map = owner.bundle_map().ok_or("source trapdoor map")?;
    let observed = || -> TestResult<_> {
        let tile = compose_tile(&map.overlay, &map.facts, DOOR).map_err(super::fixture::debug)?;
        Ok(tile.into_iter().flat_map(|tile| tile.entries)
            .map(|entry| entry.facts.appearance_id)
            .filter(|appearance| matches!(appearance, 11257 | 11258))
            .collect::<Vec<_>>())
    };
    let runtime = owner.runtime.lock().await;
    runtime.owner_fence().map_err(super::fixture::debug)?;
    for (now, expected) in [(9_999_999, vec![]), (10_000_000, vec![11257]),
        (19_999_999, vec![11257]), (20_000_000, vec![11258]), (20_000_000, vec![11258]),
        (30_000_000, vec![11257])] {
        owner.drain_world_movement_timers(now);
        assert_eq!(observed()?, expected, "at {now} microseconds");
    }
    Ok(())
}
