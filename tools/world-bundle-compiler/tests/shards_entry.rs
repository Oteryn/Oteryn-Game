//! W16 Rakesh source geometry survives real B3 -> native floor conversion.
use oteryn_world_bundle_compiler::{
    b3,
    bundle::{BuildClass, Identity},
    compile::{self, Input},
    project::{Families, TravelRoute},
    sector::{Budget, TileLimits},
    source,
};
use std::collections::{BTreeSet, VecDeque};
use std::path::Path;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn shards_rakesh_source_portals_room_floor_and_fixed_anchor_are_reachable() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(129, 128)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let bytes = std::fs::read(root.join("content/world/placements/region-z08-x129-y128.b3"))?;
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
    assert_eq!(region.z, 8);
    let mut pure = BTreeSet::new();
    for (_, _, tiles) in &region.sectors {
        for tile in tiles {
            if (33034..=33051).contains(&tile.x)
                && (32983..=33000).contains(&tile.y)
                && tile.items.len() == 1
                && palette[tile.items[0].palette as usize] == "oteryn:item.tibia.i104"
            {
                pure.insert((tile.x, tile.y));
            }
        }
    }
    assert!(
        pure.contains(&(33042, 32993)),
        "the native fixed boss anchor is exact source pure floor104, not a relocated donor spawn"
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
    if let Some(path) = std::env::var_os("OTERYN_SRV7_RAKESH_BUNDLE") {
        std::fs::write(path, &compiled.bytes)?;
    }
    let native = oteryn_world_bundle::bundle::read(&compiled.bytes)?;
    let at = |x, y| {
        native
            .sectors
            .iter()
            .filter(|s| s.floor == -8)
            .flat_map(|s| &s.tiles)
            .find(|t| t.x == x && t.y == y)
    };
    for (from, to) in [
        ((33039, 32964), (33045, 32988)),
        ((33043, 32986), (33040, 32962)),
    ] {
        let tile = at(from.0, from.1).ok_or("source portal missing from native bundle")?;
        assert!(
            tile.items
                .iter()
                .any(|i| i.attrs.teleport == Some((to.0, to.1, (-8_i8) as u8))),
            "native teleport floor byte must be−8, never legacy+8"
        );
    }
    assert!(at(33042, 32993).is_some());
    let routes = Families {
        routes: (33034..=33051)
            .flat_map(|x| (32983..=33000).map(move |y| (x, y, 8)))
            .chain([(33040, 32962, 8)])
            .enumerate()
            .map(|(n, destination)| TravelRoute {
                service: "oteryn:service.shards_source_entry_validation".into(),
                route: format!("landing_{n}"),
                destination,
            })
            .collect(),
        ..Families::default()
    };
    let (_, report) = compile::realize_npcs(
        &Input {
            families: &routes,
            ..input
        },
        &registry,
        &native.spawns,
    )?;
    let held: BTreeSet<_> = report.routes_held.iter().map(|r| r.destination).collect();
    for destination in [(33045, 32988, 8), (33042, 32993, 8), (33040, 32962, 8)] {
        assert!(
            !held.contains(&destination),
            "required source landing refused: {destination:?}: {report:#?}"
        );
    }
    // Ask the same production compiler classifier about every source tile. Decorative
    // stacks remain intact; only genuinely admitted destinations enter the path graph.
    assert!(
        report.routes_held.iter().any(
            |r| r.destination == (33043, 32986, 8) && r.reason == compile::HoldReason::Teleport
        ),
        "source return is a real teleport, not an ordinary travel landing"
    );
    let mut walkable: BTreeSet<_> = routes
        .routes
        .iter()
        .map(|r| r.destination)
        .filter(|p| !held.contains(p))
        .map(|(x, y, _)| (x, y))
        .collect();
    walkable.insert((33043, 32986));
    let mut visited = BTreeSet::from([(33045, 32988)]);
    let mut queue = VecDeque::from([(33045_u16, 32988_u16)]);
    while let Some((x, y)) = queue.pop_front() {
        for (dx, dy) in [(1_i32, 0_i32), (-1, 0), (0, 1), (0, -1)] {
            let next = (
                u16::try_from(i32::from(x) + dx)?,
                u16::try_from(i32::from(y) + dy)?,
            );
            if walkable.contains(&next) && visited.insert(next) {
                queue.push_back(next);
            }
        }
    }
    assert!(
        visited.contains(&(33042, 32993)) && visited.contains(&(33043, 32986)),
        "real compiler-admitted source cells must join entry, fixed anchor and manual return"
    );
    assert_eq!(report.routes, 325);
    Ok(())
}
