//! Materialized-pot scenarios share the real native wire fixture.
use super::{TestResult, run_case};

#[derive(Clone, Copy, Debug)]
pub(super) enum Case {
    GroundPlant,
    GroundDrop,
    GroundDropRemainder,
    OrdinaryGroundDrop,
    GroundPreserve,
    NestedCarriedPlant,
    CarriedPreserve,
}
impl Case {
    pub(super) fn preserve(self) -> bool {
        matches!(self, Self::GroundPreserve | Self::CarriedPreserve)
    }
    pub(super) fn carried(self) -> bool {
        matches!(self, Self::NestedCarriedPlant | Self::CarriedPreserve)
    }
    pub(super) fn nested(self) -> bool {
        matches!(self, Self::NestedCarriedPlant)
    }
}
#[test]
fn materialized_pot_ordinary_seed_drop_wire_commit_rollback_replay() -> TestResult {
    run_case(Case::OrdinaryGroundDrop)
}
#[test]
fn materialized_pot_seed_drop_retains_ground_remainder_and_replays() -> TestResult {
    run_case(Case::GroundDropRemainder)
}
#[test]
fn materialized_pot_seed_drop_wire_commit_rollback_replay() -> TestResult {
    run_case(Case::GroundDrop)
}
#[test]
fn ice_seed_use_materialized_ground_pot_wire_receipt_and_absolute_chain() -> TestResult {
    run_case(Case::GroundPlant)
}
#[test]
fn ice_seed_preserves_materialized_ground_plant_and_grants_once() -> TestResult {
    run_case(Case::GroundPreserve)
}
#[test]
fn ice_seed_from_open_nested_container_plants_carried_pot() -> TestResult {
    run_case(Case::NestedCarriedPlant)
}
#[test]
fn ice_seed_preserves_carried_plant_without_mutating_its_identity() -> TestResult {
    run_case(Case::CarriedPreserve)
}
