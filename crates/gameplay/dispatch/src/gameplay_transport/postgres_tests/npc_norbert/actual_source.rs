//! Original compiled map and NPCframe; no source NPC is reseeded beside a synthetic actor.
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemType};
pub(super) const ENTRY: (i32, i32, i16) = (32953, 32103, 6);
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
        .find(|n| n.key == "oteryn:npc.norbert")
        .ok_or("real Norbert source placement")?;
    assert_eq!(edvard.placements.len(), 1);
    assert_eq!(
        (
            edvard.placements[0].x,
            edvard.placements[0].y,
            edvard.placements[0].floor
        ),
        (32952, 32103, -6)
    );
    // Keep the representative's original source placement; the full compiled
    // map remains unchanged. One player and this one NPC occupy two slots.
    table.npcs.retain(|npc| npc.key == "oteryn:npc.norbert");
    let pins = crate::map::BundlePins {
        digest: b.digest,
        project_format_version: b.manifest.identity.project_format_version.clone(),
        world_schema_version: b.manifest.identity.world_schema_version.clone(),
        content_revision: b.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let index = native.item_index().ok_or("retailer native Item index")?;
    let boot=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:32953,y:32103,floor:-6}},world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key|{
 let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|p|p.record());
 Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)?;
    Ok((boot, table))
}
