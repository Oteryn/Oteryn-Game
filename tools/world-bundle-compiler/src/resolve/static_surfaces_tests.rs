use super::*;

#[test]
fn srx3_source_static_world_objects_compile_without_inventory_identity() -> Result<(), Error> {
    let mut registry = Registry::default();
    registry.add_catalogue(
        "WorldObject",
        include_bytes!("../../../../content/world/objects/objects-12932-13108.json"),
    )?;
    registry.seal()?;
    let key = "oteryn:world-object.tibia.i10649";
    assert!(
        matches!(
            registry.resolve(key),
            Resolution::Resolved(Family::Terrain, _)
        ),
        "the actual closed source static object needs a non-inventory map route"
    );
    assert_eq!(
        registry.terrain(key)?,
        Some(Terrain {
            kind: TerrainKind::Common,
            walkable: None,
            ground_speed: None,
            blocks_projectile: Some(false),
        })
    );
    assert!(registry.solid(key));
    assert!(registry.solid_stated(key));
    assert!(!registry.solid("oteryn:world-object.tibia.i25181"));
    assert_eq!(
        registry.resolve("oteryn:item.tibia.i10649"),
        Resolution::Unknown
    );
    Ok(())
}
