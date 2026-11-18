use super::{TestResult, soulwarmv_reward_portal::portal_steps};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[test]
fn soulwarmv_megalomania_gate_step_reads_distinct_miniboss_flags_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    portal_steps(
        "soulwarmv_megalomania_gate",
        [33612, 31430, 10],
        StepDirection::West,
        &[
            (
                250,
                0b01111,
                true,
                [33612, 31430, 10],
                Some("You still need to defeat:\nGoshnar's Greed"),
            ),
            (
                250,
                0b11110,
                false,
                [33612, 31430, 10],
                Some("You still need to defeat:\nGoshnar's Malice"),
            ),
            (
                249,
                0b11111,
                false,
                [33612, 31430, 10],
                Some("You are not allowed to enter here."),
            ),
            (250, 0b11111, false, [33683, 31634, 14], None),
        ],
    )
}
