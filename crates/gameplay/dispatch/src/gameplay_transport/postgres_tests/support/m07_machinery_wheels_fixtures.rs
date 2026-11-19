use super::TestResult;

use crate::content::native_gameplay::NativeGameplayState;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) fn start()->Position {Position{x:32964,y:31813,floor:-7}}

pub(super) fn world(native:&NativeGameplayState)->TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{NativeCrop,ExpectedTile,native_scene_with_appearances};
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
