//! Original compiled map and NPCframe; no source NPC is reseeded beside a synthetic actor.
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemType};
pub(super) fn world(
    world: crate::foundation::WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<(crate::map::boot::BundleWorld, NpcPlacementTable)> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_HORSE_BUNDLE").ok_or("actual retailer source bundle")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let mut table = NpcPlacementTable {
        npcs: b
            .npcs
            .npcs
            .iter()
            .map(|n| NpcPlacements {
                key: n.key.clone(),
                placements: n
                    .placements
                    .iter()
                    .map(|p| NpcPlacement {
                        x: p.x,
                        y: p.y,
                        floor: p.floor,
                        direction: match p.direction {
                            oteryn_world_bundle::npc::Direction::North => MovementFacing::North,
                            oteryn_world_bundle::npc::Direction::East => MovementFacing::East,
                            oteryn_world_bundle::npc::Direction::South => MovementFacing::South,
                            oteryn_world_bundle::npc::Direction::West => MovementFacing::West,
                        },
                    })
                    .collect(),
            })
            .collect(),
    };
    let edvard = table
        .npcs
        .iter()
        .find(|n| n.key == "oteryn:npc.hanna")
        .ok_or("real Hanna source placement")?;
    assert_eq!(edvard.placements.len(), 1);
    assert_eq!(
        (
            edvard.placements[0].x,
            edvard.placements[0].y,
            edvard.placements[0].floor
        ),
        (32408, 32220, -7)
    );
    for (key, cell) in [
        ("oteryn:npc.lynda", (32331, 32199, -7)),
        ("oteryn:npc.norbert", (32952, 32103, -6)),
    ] {
        let placements = &table
            .npcs
            .iter()
            .find(|n| n.key == key)
            .ok_or("source coupled wedding NPC")?
            .placements;
        assert_eq!(placements.len(), 1);
        assert_eq!(
            (placements[0].x, placements[0].y, placements[0].floor),
            cell
        );
    }
    table.npcs.retain(|n| {
        matches!(
            n.key.as_str(),
            "oteryn:npc.hanna" | "oteryn:npc.norbert" | "oteryn:npc.lynda"
        )
    });
    let pins = crate::map::BundlePins {
        digest: b.digest,
        project_format_version: b.manifest.identity.project_format_version.clone(),
        world_schema_version: b.manifest.identity.world_schema_version.clone(),
        content_revision: b.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let index = native.item_index().ok_or("retailer native Item index")?;
    let boot=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:32408,y:32221,floor:-7}},world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key|{
 let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|p|p.record());
 Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)?;
    Ok((boot, table))
}
