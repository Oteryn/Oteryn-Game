//! M07 catapults use the same retained-tool, level and personal-credit authority as ballistae.
use super::TestResult;
use crate::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

const SOURCE:Position=Position{x:32909,y:31733,floor:-7};
pub(super) fn start()->Position {Position{x:32910,y:31733,floor:-7}}

#[test]
fn m07_machinery_postgres_catapult_retains_tool_checks_level_and_resets()->TestResult {
    super::m07_machinery_burn::run("m07/machinery-catapult-01",SOURCE)
}
#[test]
fn m07_machinery_postgres_catapult_refuses_level49_without_credit_or_tool_charge()->TestResult {
    super::m07_machinery_burn::run("m07/machinery-catapult-01-level49",SOURCE)
}

pub(super) fn world(native:&NativeGameplayState)->TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{NativeCrop,ExpectedTile,native_scene_with_appearances};
    native_scene_with_appearances(native,vec![NativeCrop{
        bytes:include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content/world/placements/region-z07-x128-y123.b3")),
        region:(7,128,123),bounds:(32907,31731,32912,31735),start:(32910,31733),
        expected:vec![
            ExpectedTile{position:(32909,31733),ids:vec![4528,5608],counted_entry:None},
            ExpectedTile{position:(32909,31732),ids:vec![4517,5606],counted_entry:None},
            ExpectedTile{position:(32908,31733),ids:vec![4515,5607],counted_entry:None},
            ExpectedTile{position:(32908,31732),ids:vec![4519,5605],counted_entry:None},
            ExpectedTile{position:(32910,31733),ids:vec![4515],counted_entry:None},
        ],
    }],&[],&[2113])
}
