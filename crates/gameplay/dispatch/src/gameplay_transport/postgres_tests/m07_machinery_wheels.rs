//! Six actual two-part war wheels share the retained-tool and personal-credit authority.
use super::TestResult;
use crate::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

const SOURCE:Position=Position{x:32964,y:31814,floor:-7};
pub(super) fn start()->Position {Position{x:32964,y:31813,floor:-7}}

#[test]
fn m07_machinery_postgres_wheel_retains_tool_checks_level_and_resets()->TestResult {
    super::m07_machinery_burn::run("m07/machinery-wheel-01",SOURCE)
}
#[test]
fn m07_machinery_postgres_wheel_refuses_level49_without_credit_or_tool_charge()->TestResult {
    super::m07_machinery_burn::run("m07/machinery-wheel-01-level49",SOURCE)
}

pub(super) fn world(native:&NativeGameplayState)->TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{NativeCrop,ExpectedTile,native_scene_with_appearances};
    native_scene_with_appearances(native,vec![NativeCrop{
        bytes:include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content/world/placements/region-z07-x128-y124.b3")),
        region:(7,128,124),bounds:(32964,31813,32966,31815),start:(32964,31813),
        expected:vec![
            ExpectedTile{position:(32964,31814),ids:vec![4406,7775],counted_entry:None},
            ExpectedTile{position:(32965,31814),ids:vec![4403,4663,7776],counted_entry:None},
            ExpectedTile{position:(32964,31813),ids:vec![4407],counted_entry:None},
        ],
    }],&[],&[2113])
}
