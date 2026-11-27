use oteryn_world_bundle_compiler::{project::Families, source};
use std::path::Path;

#[test]
fn crystal_creature_definitions_resolve_in_the_spawn_loader()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/creatures/definitions");
    let mut families = Families::default();
    for shard in source::shards(&root, "creatures-")? {
        families.add_creatures(&shard)?;
    }
    for name in [
        "ayana_the_crimson_curse",
        "channeling_earl_osam",
        "common_beetle",
        "crate",
        "elderbloodjaw",
        "empowered_energised_raubritter",
        "empowered_frozen_raubritter",
        "empowered_poisoned_raubritter",
        "empowered_raging_raubritter",
        "enraged_arbaziloth",
        "enusat_the_onyx_wing",
        "foxmouse",
        "gorga",
        "gralvalon",
        "grove_guardian",
        "herd_weevil",
        "imp_minion",
        "malvaroth",
        "morbol",
        "quara_kindler",
        "rootthing_cocoon",
        "rootthing_conservator",
        "target_dummy",
        "the_forgemaster",
        "twisterror",
        "unleashed_hex",
        "weakened_arbaziloth",
    ] {
        let key = format!("oteryn:creature.{name}");
        assert!(families.creatures.contains_key(&key), "missing {key}");
    }
    Ok(())
}
