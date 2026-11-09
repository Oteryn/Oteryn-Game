//! Use retained regions and production Item policies, not synthetic walkable replacements.
use super::*;

pub(super) fn load(
    native: &crate::content::native_gameplay::NativeGameplayState,
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<(
    crate::map::boot::BundleWorld,
    crate::foundation::NpcPlacementTable,
)> {
    use oteryn_world_bundle_compiler::{bundle, compile, source};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let project =
        source::project_regions(&root, &[(124, 124), (126, 126), (127, 126), (127, 127)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let (seed, mut pins) = crate::map::boot::tests::bundle();
    let identity = bundle::read(&seed)?.manifest.identity;
    let input = compile::Input {
        regions: &project.regions,
        palette: &palette,
        identity,
        world: source::world_extent(&root)?,
        build_class: bundle::BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    let decoded = bundle::read(&compiled.bytes)?;
    let anna = decoded
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == ANNA)
        .ok_or("source Anna admission")?;
    let p = anna
        .placements
        .iter()
        .find(|p| (p.x, p.y, p.floor) == (32552, 32497, -7))
        .ok_or("canonical Anna anchor")?;
    assert_eq!(p.direction, oteryn_world_bundle::spawn::Direction::South);
    let table = crate::foundation::NpcPlacementTable {
        npcs: vec![crate::foundation::NpcPlacements {
            key: ANNA.into(),
            placements: vec![crate::foundation::NpcPlacement {
                x: p.x,
                y: p.y,
                floor: p.floor,
                direction: crate::foundation::MovementFacing::South,
            }],
        }],
    };
    let bad: Vec<_> = compiled
        .diagnostics
        .iter()
        .filter(|d| {
            d.floor == -7
                && [(32552_u16, 32497_u16), (31938, 31928), (32534, 32513)]
                    .iter()
                    .any(|(x, y)| d.x.abs_diff(*x) <= 3 && d.y.abs_diff(*y) <= 3)
        })
        .map(|d| d.key.as_str())
        .collect();
    assert!(
        bad.is_empty(),
        "source anchors cannot silently omit provisional geometry: {bad:?}"
    );
    pins.digest = compiled.digest;
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    let boot = crate::map::boot::BootPins {
        map_revision: crate::map::overlay::map_revision(&base),
        bundle: pins,
        start: crate::map::overlay::TilePos {
            x: 32534,
            y: 32513,
            floor: -7,
        },
    };
    let index = native.item_index().ok_or("native Item index")?;
    let map = crate::map::boot::boot(&compiled.bytes,&boot,world,channel,|key| {
        use crate::content::{ReferenceItemField,ReferenceItemType};
        let revision = index.revision(key)?;
        let reference = index.definition_ref("Item",key,revision)?;
        let profile = native.item_policy(key,revision).map(|p|p.record());
        let pickupable = profile.is_some_and(|p| matches!(&p.semantics.physical,ReferenceItemField::Known(v) if v.pickupable==ReferenceItemField::Known(true)));
        let door = profile.is_some_and(|p| matches!(&p.semantics.classification,ReferenceItemField::Known(v) if v.item_type==ReferenceItemField::Known(ReferenceItemType::Door)));
        let attrs = profile.map(|p|&p.attributes);
        Some(crate::map::facts::ItemDefinition {
            reference,pickupable,door,solid:attrs.and_then(|a|a.blocks_movement),
            blocks_projectile:attrs.and_then(|a|a.blocks_projectile).unwrap_or(true),
        })
    }).map_err(debug)?;
    Ok((map, table))
}
