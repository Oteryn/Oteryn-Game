//! Test data: the ordinary map fixture with one source-numbered House door.
use super::{TestResult, fixture::debug};
pub(super) fn house_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::boot::{self, tests as fixture};
    use oteryn_world_bundle::bundle;
    let (bytes, mut pins) =
        fixture::bundle_with_items("oteryn:item.tibia.i1629", "oteryn:item.tibia.i1000");
    let mut data = bundle::read(&bytes).map_err(debug)?;
    let tile = data
        .sectors
        .iter_mut()
        .flat_map(|s| s.tiles.iter_mut())
        .find(|t| t.x == 6)
        .ok_or("door tile")?;
    tile.house = 3437;
    tile.items.last_mut().ok_or("door entry")?.attrs.door = Some(42);
    let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
        &data.manifest,
        &data.sectors,
        &data.spawns,
        &data.npcs,
    )
    .map_err(debug)?;
    pins.digest = bundle::read(&bytes).map_err(debug)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot::boot(
        &bytes,
        &boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: 2,
                y: 0,
                floor: -7,
            },
        },
        world,
        channel,
        fixture::door_items,
    )
    .map_err(debug)
}
