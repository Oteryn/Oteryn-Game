//! W05d: explicitly derived reward-room layout and real native map qualification.
use oteryn_world_bundle_compiler::{
    b3,
    bundle::{BuildClass, Identity},
    compile::{self, Input},
    project::{Families, TravelRoute},
    sector::{Budget, TileLimits},
    source,
};
use std::path::Path;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn srv7_pumpkin_derived_reward_room_has_real_base_body_portal_and_temple_path() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(root.join("content/world/placements/region-z15-x130-y126.b3"))?;
    let region = b3::decode_region(
        &bytes,
        TileLimits {
            max_entries: 4096,
            max_text_bytes: 16 * 1024,
        },
        &mut Budget {
            tiles: 65536,
            entries: 1_000_000,
        },
    )?;
    let room = region
        .sectors
        .iter()
        .flat_map(|(_, _, tiles)| tiles)
        .filter(|t| (33312..=33322).contains(&t.x) && (32410..=32420).contains(&t.y))
        .collect::<Vec<_>>();
    assert_eq!(
        room.len(),
        121,
        "the explicitly derived ordinary-World room must actually exist, never a fabricated successful landing"
    );
    assert!(
        room.iter()
            .all(|t| t.flags == 0 && t.house == 0 && t.zones.is_empty()),
        "no source claim about an unproven protection zone"
    );
    let project = source::project_regions(&root, &[(129, 126), (130, 126)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let body = room
        .iter()
        .find(|t| (t.x, t.y) == (33317, 32414))
        .ok_or("derived base body cell")?;
    assert_eq!(
        body.items
            .iter()
            .filter(|i| palette[i.palette as usize] == "oteryn:item.tibia.i12816")
            .count(),
        1,
        "one source-qualified base reward object, never a second dead Creature or corpse MINT"
    );
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: source::world_extent(&root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    if let Some(path) = std::env::var_os("OTERYN_SRV7_PUMPKIN_BUNDLE") {
        std::fs::write(path, &compiled.bytes)?;
    }
    let native = oteryn_world_bundle::bundle::read(&compiled.bytes)?;
    let at = |x, y, floor| {
        native
            .sectors
            .iter()
            .filter(|s| s.floor == floor)
            .flat_map(|s| &s.tiles)
            .find(|t| t.x == x && t.y == y)
    };
    let surface = at(33317, 32415, -7).ok_or("exact source surface entry")?;
    assert!(
        surface
            .items
            .iter()
            .any(
                |i| native.manifest.palette[i.palette as usize].key == "oteryn:item.tibia.i1949"
                    && i.attrs.teleport.is_none()
            ),
        "surface portal presentation never supplies an unconditional generic teleport"
    );
    let exit = at(33317, 32419, -15).ok_or("derived room exit")?;
    assert!(
        exit.items
            .iter()
            .any(|i| i.attrs.teleport == Some((33213, 32454, (-1_i8) as u8))),
        "native return must bind exact source Darashia temple/floor"
    );
    let families = Families {
        routes: [
            (33317, 32417, 15),
            (33317, 32415, 15),
            (33213, 32454, 1),
            (33115, 32398, 7),
            (33095, 32381, 7),
            (33092, 32418, 7),
            (33317, 32415, 7),
        ]
        .into_iter()
        .enumerate()
        .map(|(n, destination)| TravelRoute {
            service: "oteryn:service.pumpkin_room_validation".into(),
            route: format!("landing_{n}"),
            destination,
        })
        .collect(),
        ..Families::default()
    };
    let (_, report) = compile::realize_npcs(
        &Input {
            families: &families,
            ..input
        },
        &registry,
        &native.spawns,
    )?;
    assert_eq!(
        report.routes, 7,
        "all three source boss cells and four portal/room/temple approaches must execute"
    );
    assert!(
        report.routes_held.is_empty(),
        "actual three source boss spawns/surface portal/room entry/body approach/temple geometry must admit: {report:#?}"
    );
    Ok(())
}
