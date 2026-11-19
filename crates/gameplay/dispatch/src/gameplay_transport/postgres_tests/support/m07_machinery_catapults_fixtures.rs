use super::TestResult;

use crate::content::native_gameplay::NativeGameplayState;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) fn start()->Position {Position{x:32910,y:31733,floor:-7}}

pub(super) fn world(native:&NativeGameplayState)->TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{NativeCrop,ExpectedTile,native_scene_with_appearances};
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
