use oteryn_gameplay_world_change::timed::{EventKind, EventState, Input, Phase};

#[test]
fn wevents_calendar_and_shared_objects() {
    let mut flower = EventState::new(0, 3_600_000);
    assert_eq!(
        flower.advance(EventKind::FlowerMonth, 0, Input::Tick, 0),
        Ok(true)
    );
    assert_eq!(flower.wave, 1);
    assert_eq!(
        flower.advance(EventKind::FlowerMonth, 899_999, Input::Tick, 0),
        Ok(false)
    );
    assert_eq!(
        flower.advance(EventKind::FlowerMonth, 900_000, Input::Tick, 0),
        Ok(true)
    );
    assert_eq!(flower.wave, 2);
    let mut jars = EventState::new(0, 86_400_000);
    for aid in 50006..=50010 {
        assert_eq!(
            jars.advance(EventKind::MummysCurse, 0, Input::Jar(aid), 0),
            Ok(true)
        );
    }
    assert_eq!(jars.phase, Phase::Victory);
    assert_eq!(jars.boss_spawns, 1);
    assert_eq!(
        jars.advance(EventKind::MummysCurse, 0, Input::Jar(50010), 0),
        Ok(false)
    );
}
