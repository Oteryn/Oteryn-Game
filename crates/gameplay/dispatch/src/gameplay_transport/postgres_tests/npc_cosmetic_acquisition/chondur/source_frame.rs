//! Actual exported source frame; no synthesized geometry, NPC or producer.
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemType};
use crate::foundation::MovementLocalPosition as Pos;
pub(super) struct Frame {
    pub world: crate::map::boot::BundleWorld,
    pub start: Pos,
    pub stampor: Option<(Pos, u32)>,
    pub spawns: oteryn_world_bundle::spawn::Table,
    pub npcs: NpcPlacementTable,
}
pub(super) fn load(
    world: crate::foundation::WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
    final_trade: bool,
) -> TestResult<Frame> {
    let bytes = std::fs::read(
        std::env::var_os(if final_trade {
            "OTERYN_SRV7_CHONDUR_BUNDLE"
        } else {
            "OTERYN_SRV7_STAMPOR_BUNDLE"
        })
        .ok_or("actual source frame artifact")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let pure = |x: i32, y: i32, floor: i8| {
        b.sectors
            .iter()
            .filter(|s| s.floor == floor)
            .flat_map(|s| &s.tiles)
            .find(|t| i32::from(t.x) == x && i32::from(t.y) == y)
            .is_some_and(|t| {
                t.house == 0
                    && t.items.len() == 1
                    && t.items[0].depth == 0
                    && b.manifest
                        .palette
                        .get(t.items[0].palette as usize)
                        .is_some_and(|p| {
                            p.terrain.is_some_and(|t| {
                                t.kind == oteryn_world_bundle::bundle::TerrainKind::Ground
                                    && t.walkable == Some(true)
                            })
                        })
            })
    };
    let (start, stampor) = if final_trade {
        let p = Pos {
            x: 32361,
            y: 32551,
            floor: 7,
        };
        assert!(pure(p.x, p.y, -7), "actual pure486 source Chondur talkcell");
        let n = b
            .npcs
            .npcs
            .iter()
            .find(|n| n.key == "oteryn:npc.chondur")
            .ok_or("actual admitted Chondur source NPC")?;
        assert_eq!(n.placements.len(), 1);
        assert_eq!(
            (n.placements[0].x, n.placements[0].y, n.placements[0].floor),
            (32361, 32550, -7)
        );
        (p, None)
    } else {
        let mut chosen = None;
        'source: for s in &b.spawns.sources {
            for p in &s.points {
                if b.spawns
                    .creatures
                    .get(p.creature as usize)
                    .is_none_or(|c| c.key != "oteryn:creature.stampor")
                {
                    continue;
                }
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let x = i32::from(p.x) + dx;
                    let y = i32::from(p.y) + dy;
                    if pure(x, y, s.floor) {
                        chosen = Some((
                            Pos {
                                x,
                                y,
                                floor: i16::from(s.floor.unsigned_abs()),
                            },
                            Some((
                                Pos {
                                    x: i32::from(p.x),
                                    y: i32::from(p.y),
                                    floor: i16::from(s.floor.unsigned_abs()),
                                },
                                p.respawn_ms,
                            )),
                        ));
                        break 'source;
                    }
                }
            }
        }
        chosen.ok_or("original Stampor source point with source-qualified adjacent start")?
    };
    let npcs = NpcPlacementTable {
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
    let pins = crate::map::BundlePins {
        digest: b.digest,
        project_format_version: b.manifest.identity.project_format_version.clone(),
        world_schema_version: b.manifest.identity.world_schema_version.clone(),
        content_revision: b.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let index = native.item_index().ok_or("native Item index")?;
    let boot=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:u16::try_from(start.x)?,y:u16::try_from(start.y)?,floor:-i8::try_from(start.floor)?}},world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key|{
  let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|p|p.record());Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p)if p.pickupable==F::Known(true))),door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c)if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)?;
    Ok(Frame {
        world: boot,
        start,
        stampor,
        spawns: b.spawns,
        npcs,
    })
}
