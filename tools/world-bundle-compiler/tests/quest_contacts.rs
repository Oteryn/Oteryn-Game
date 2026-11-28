//! Quest NPC placements must survive the production world compiler.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{Input, realize_npcs, realize_spawns},
    source,
};
use std::error::Error;

#[test]
fn quest_contacts_are_realized_on_their_donor_tiles() -> Result<(), Box<dyn Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(128, 127), (129, 124)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: source::world_extent(&root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (spawns, _) = realize_spawns(&input, &registry)?;
    let (table, report) = realize_npcs(&input, &registry, &spawns)?;
    for (key, expected) in [
        ("oteryn:npc.tereban", &[(33170, 31807, -6)][..]),
        ("oteryn:npc.tarun", &[(32976, 32715, -7)][..]),
    ] {
        let npc = table
            .npcs
            .iter()
            .find(|npc| npc.key == key)
            .ok_or_else(|| {
                format!(
                    "contact not realized {key}: {:?}",
                    report
                        .held
                        .iter()
                        .filter(|r| r.npc == key)
                        .collect::<Vec<_>>()
                )
            })?;
        let mut cells: Vec<_> = npc
            .placements
            .iter()
            .map(|cell| (cell.x, cell.y, cell.floor))
            .collect();
        cells.sort_unstable();
        assert_eq!(cells, expected, "{key}");
    }
    Ok(())
}

#[test]
fn fathers_burden_materials_are_nested_in_their_source_containers() -> Result<(), Box<dyn Error>> {
    let materials = [
        (31987, 31223, -7, 2469, 11547),
        (32815, 32502, -8, 2469, 11551),
        (33215, 31413, -10, 11591, 11545),
        (33235, 32803, -12, 1994, 11552),
        (32122, 31912, -14, 1983, 11546),
        (32636, 31985, -14, 2469, 11549),
    ];
    let regions = materials.map(|(x, y, _, _, _)| (x / 256, y / 256));
    let bundle = compile_regions(&regions)?;
    for (x, y, floor, container, material) in materials {
        let tile = bundle
            .sectors
            .iter()
            .filter(|sector| sector.floor == floor)
            .flat_map(|sector| &sector.tiles)
            .find(|tile| (tile.x, tile.y) == (x, y))
            .ok_or("material source tile")?;
        let key = |item: &oteryn_world_bundle::sector::Item| {
            bundle.manifest.palette[item.palette as usize].key.as_str()
        };
        let parent = format!("oteryn:item.tibia.i{container}");
        let child = format!("oteryn:item.tibia.i{material}");
        let contents: Vec<_> = tile
            .items
            .iter()
            .skip_while(|item| item.depth != 0 || key(item) != parent)
            .skip(1)
            .take_while(|item| item.depth != 0)
            .map(|item| (key(item), item.depth, item.attrs.count.unwrap_or(1)))
            .collect();
        assert_eq!(
            contents,
            [(child.as_str(), 1, 1)],
            "container at {x},{y},{floor}"
        );
    }
    Ok(())
}

#[test]
fn dragon_hunt_explorer_corpses_have_their_start_action() -> Result<(), Box<dyn Error>> {
    let bundle = compile_regions(&[(128, 127)])?;
    for (x, y) in [(33007, 32607), (32991, 32642)] {
        let tile = bundle
            .sectors
            .iter()
            .filter(|sector| sector.floor == -8)
            .flat_map(|sector| &sector.tiles)
            .find(|tile| (tile.x, tile.y) == (x, y))
            .ok_or("dead explorer tile")?;
        let corpse = tile
            .items
            .iter()
            .find(|item| {
                item.depth == 0
                    && bundle.manifest.palette[item.palette as usize].key
                        == "oteryn:item.tibia.i21412"
            })
            .ok_or("dead explorer")?;
        assert_eq!(corpse.attrs.action, Some(50806), "{x},{y}");
    }
    Ok(())
}

fn compile_regions(
    regions: &[(u16, u16)],
) -> Result<oteryn_world_bundle::bundle::Bundle, Box<dyn Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, regions)?;
    let (registry, palette) = source::registry(&root, &project)?;
    let output = oteryn_world_bundle_compiler::compile::compile(
        &Input {
            regions: &project.regions,
            palette: &palette,
            identity: Identity::default(),
            world: source::world_extent(&root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &project.families,
        },
        &registry,
    )?;
    Ok(oteryn_world_bundle::bundle::read(&output.bytes)?)
}
