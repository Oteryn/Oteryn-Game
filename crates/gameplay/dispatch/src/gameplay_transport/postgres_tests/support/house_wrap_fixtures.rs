use super::fixture::debug;

use super::TestResult;

pub(super) fn wrap_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    wrap_bundle_with_static_decorations(world, channel, &[], false)
}

pub(super) fn walkable_house_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    wrap_bundle_with_static_decorations(world, channel, &[], true)
}

pub(super) fn static_decoration_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    static_decoration_bundle_with(world,channel,&[("oteryn:item.tibia.i5302",7,0)])
}

pub(super) fn static_timed_decoration_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    static_decoration_bundle_with(world,channel,&[("oteryn:item.tibia.i35949",7,0)])
}

pub(super) fn static_bed_decoration_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    static_decoration_bundle_with(world,channel,&[("oteryn:item.tibia.i2491",7,0),("oteryn:item.tibia.i2492",7,1)])
}

struct StaticDecoration {
    key: &'static str,
    x: u16,
    y: u16,
    definition: crate::map::facts::ItemDefinition,
}

fn static_decoration_bundle_with(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    declarations: &[(&'static str,u16,u16)],
) -> TestResult<crate::map::boot::BundleWorld> {
    let (controller, _) = crate::content::native_gameplay::test_support::activated_with_loot_tables(world,None);
    let native=controller.active().and_then(|active|active.native_gameplay()).ok_or("native generation")?;
    let index=native.item_index().ok_or("Item index")?;
    let mut extras=Vec::with_capacity(declarations.len());
    for &(key,x,y) in declarations {
        let reference=index.definition_ref("Item",key,"definition-r1").ok_or("decoration reference")?;
        let policy=native.item_policy(key,"definition-r1").ok_or("decoration policy")?.record();
        extras.push(StaticDecoration {key,x,y,definition:crate::map::facts::ItemDefinition {
            reference,solid:policy.attributes.blocks_movement,
            blocks_projectile:policy.attributes.blocks_projectile.ok_or("decoration projectile fact")?,
            pickupable:false,door:false }});
    }
    wrap_bundle_with_static_decorations(world, channel, &extras, false)
}

fn wrap_bundle_with_static_decorations(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    extras: &[StaticDecoration],
    adjacent_ground: bool,
) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::boot::{self, tests as fixture};
    use oteryn_world_bundle::bundle;
    let (bytes, mut pins) =
        fixture::bundle_with_items("oteryn:item.tibia.i1629", "oteryn:item.tibia.i1000");
    let mut data = bundle::read(&bytes).map_err(debug)?;
    let sector = data.sectors.first_mut().ok_or("sector")?;
    let mut entrance = sector
        .tiles
        .iter()
        .find(|t| t.x == 3)
        .ok_or("grass tile")?
        .clone();
    let seed=sector.tiles.iter().find(|tile|tile.x==7).ok_or("House tile")?;
    let ground=seed.items.first().ok_or("House ground")?.palette;
    let flags=seed.flags;
    // Clear only the fixture seed before placing actual native decorations.
    if adjacent_ground {
        let portal = seed.items.last().ok_or("teleport appearance")?.clone();
        let mut ground_tile = seed.clone();
        ground_tile.house = 3437;
        ground_tile.items.truncate(1);
        for offset in [1, 2] {
            let mut adjacent = ground_tile.clone();
            adjacent.x += offset;
            if offset == 2 {
                let mut portal = portal.clone();
                portal.attrs.teleport = Some((32311, 32803, (-7_i8).to_ne_bytes()[0]));
                adjacent.items.push(portal);
            }
            sector.tiles.push(adjacent);
        }
    }
    for tile in &mut sector.tiles {
        if tile.x == 7 { tile.items.truncate(1); }
    }
    for extra in extras {
        let palette=if let Some(at)=data.manifest.palette.iter().position(|entry|entry.key==extra.key) {
            u32::try_from(at)?
        } else {
            let at=u32::try_from(data.manifest.palette.len())?;
            data.manifest.palette.push(bundle::PaletteEntry {key:extra.key.into(),family:bundle::Family::Item,
                id:extra.definition.reference.get()-1,terrain:None});
            at
        };
        let item=oteryn_world_bundle::sector::Item {palette,depth:0,attrs:Default::default()};
        if let Some(tile)=sector.tiles.iter_mut().find(|tile|tile.x==extra.x&&tile.y==extra.y) {
            tile.house=3437;
            tile.items.push(item);
        } else {
            sector.tiles.push(oteryn_world_bundle::sector::Tile {x:extra.x,y:extra.y,flags,house:3437,zones:Vec::new(),
                items:vec![oteryn_world_bundle::sector::Item {palette:ground,depth:0,attrs:Default::default()},item]});
        }
    }
    for tile in &mut sector.tiles {
        if tile.x == 7 {
            tile.house = 3437;
        }
        if tile.x == 6 {
            tile.house = 3437;
            tile.items.last_mut().ok_or("door")?.attrs.door = Some(42);
        }
        tile.x += 32303;
        tile.y += 32804;
    }
    if adjacent_ground {
        let mut landing = sector
            .tiles
            .iter()
            .find(|tile| tile.x == 32311)
            .ok_or("east ground")?
            .clone();
        landing.y -= 1;
        sector.tiles.push(landing);
    }
    entrance.x = 32309;
    entrance.y = 32805;
    sector.tiles.push(entrance);
    // Native sector encoding orders by (y, x), including the second bed half.
    sector.tiles.sort_by_key(|tile| (tile.y, tile.x));
    sector.sx = 1009;
    sector.sy = 1025;
    data.manifest.world.min_x = 32288;
    data.manifest.world.min_y = 32800;
    data.manifest.world.max_x = 32320;
    data.manifest.world.max_y = 32832;
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
                x: 32309,
                y: 32805,
                floor: -7,
            },
        },
        world,
        channel,
        |key| extras.iter().find(|extra|extra.key==key).map(|extra|extra.definition).or_else(||fixture::door_items(key)),
    )
    .map_err(debug)
}
