use oteryn_world_bundle::world_changes::CAPABILITY;
use oteryn_world_bundle_compiler::{bundle, compile::Input, source, world_changes};

#[test]
fn wvar_compiler_admits_water_variants_and_rejects_source_house_overlap()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project(&root)?;
    let (registry, palette) = source::registry(&root, &project)?;
    let mut input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: bundle::Identity {
            required_capabilities: vec![CAPABILITY.into()],
            ..Default::default()
        },
        world: source::world_extent(&root)?,
        build_class: bundle::BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let package = world_changes::admit(&input, &registry)?.ok_or("W1 package")?;
    package.validate()?;
    let definitions: serde_json::Value = serde_json::from_str(&package.recipes)?;
    for key in ["awash", "down_the_drain", "river_runs_deep"] {
        assert!(
            definitions
                .as_array()
                .ok_or("definitions")?
                .iter()
                .any(|d| d["key"] == key),
            "{key}"
        );
    }
    let household = house_region()?;
    let regions = [household];
    input.regions = &regions;
    assert!(
        matches!(world_changes::admit(&input,&registry),Err(oteryn_world_bundle_compiler::Error::Family(message)) if message.contains("touches house"))
    );
    input.identity.required_capabilities.clear();
    assert!(world_changes::admit(&input, &registry)?.is_none());
    Ok(())
}

fn house_region() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    let (x, y, z) = (33290u16, 31785u16, 13u8);
    let tile = Tile {
        x,
        y,
        flags: 0,
        house: 99,
        zones: Vec::new(),
        items: vec![Item {
            palette: 0,
            depth: 0,
            attrs: Attrs::default(),
        }],
    };
    let raw = oteryn_world_bundle::sector::encode(&[tile])?;
    let frame = zstd::bulk::compress(&raw, 1)?;
    let mut out = b"OTRB".to_vec();
    out.extend([1, z]);
    out.extend((x / 256).to_le_bytes());
    out.extend((y / 256).to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.push(((y / 32) % 8 * 8 + (x / 32) % 8) as u8);
    out.extend(21u32.to_le_bytes());
    out.extend(u32::try_from(frame.len())?.to_le_bytes());
    out.extend(frame);
    Ok(out)
}
