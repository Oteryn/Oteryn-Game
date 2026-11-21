//! Actual source onMove delivery, not a client Item creation request.
use super::TestResult;
use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Room;
use crate::foundation::MovementLocalPosition;

#[test]
fn srx3_vemiath_actual_source_movement_footprint_postgres() -> TestResult {
    super::srx3_rotten_advanced_fixtures::footprints(
        Room::Vemiath,
        MovementLocalPosition {
            x: 33078,
            y: 32333,
            floor: 15,
        },
        "oteryn:creature.vemiath",
    )
}
#[test]
fn srx3_chagorz_actual_source_movement_footprint_postgres() -> TestResult {
    super::srx3_rotten_advanced_fixtures::footprints(
        Room::Chagorz,
        MovementLocalPosition {
            x: 33074,
            y: 32367,
            floor: 15,
        },
        "oteryn:creature.chagorz",
    )
}
#[test]
fn srx3_bakragore_actual_source_movement_footprint_postgres() -> TestResult {
    super::srx3_rotten_advanced_fixtures::footprints(
        Room::Bakragore,
        MovementLocalPosition {
            x: 33010,
            y: 32392,
            floor: 15,
        },
        "oteryn:creature.bakragore",
    )
}

#[test]
fn review3_footprint_timer_requalifies_after_instance_restart() -> TestResult {
    super::srx3_rotten_advanced_fixtures::footprints_restart()
}
