//! Exact-before, one-tile authored corrections for ordinary spawns sharing an NPC anchor.
use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::Error;
use crate::project::{self, Families};
use crate::spawn::Direction;

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cell {
    x: u16,
    y: u16,
    floor: u8,
}

impl Cell {
    fn tuple(self) -> (u16, u16, u8) {
        (self.x, self.y, self.floor)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Before {
    creature: String,
    cell: Cell,
    direction: String,
    respawn_ms: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Repair {
    source: String,
    npc: String,
    centre: Cell,
    before: Before,
    after: Cell,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    coordinate_frame: String,
    provenance: Value,
    repairs: Vec<Repair>,
}

fn invalid(message: &str) -> Error {
    Error::Format(format!("NPC spawn repair: {message}"))
}

fn distance(a: (u16, u16, u8), b: (u16, u16, u8)) -> u16 {
    a.0.abs_diff(b.0).max(a.1.abs_diff(b.1))
}

pub(super) fn apply(root: &Path, families: &mut Families) -> Result<(), Error> {
    let path = root.join("content/npcs/spawn-anchor-repairs.json");
    if !path.exists() {
        return Ok(());
    }
    let document: Document = serde_json::from_slice(&super::read(&path)?)
        .map_err(|error| invalid(&error.to_string()))?;
    apply_document(document, families)
}

fn apply_document(document: Document, families: &mut Families) -> Result<(), Error> {
    if document.schema != "OTERYN_NPC_SPAWN_ANCHOR_REPAIRS/v1"
        || document.coordinate_frame != project::FRAME
        || !document
            .provenance
            .as_object()
            .is_some_and(|p| !p.is_empty())
        || document.repairs.is_empty()
        || document.repairs.len() > 64
    {
        return Err(invalid("schema, frame, provenance or count"));
    }
    // Validate the complete successor before replacing any captured source in memory.
    let mut spawns = families.spawns.clone();
    let mut seen = BTreeSet::new();
    for repair in document.repairs {
        let before = repair.before.cell.tuple();
        let after = repair.after.tuple();
        let centre = repair.centre.tuple();
        if before.2 > 15
            || before.2 != after.2
            || before.2 != centre.2
            || distance(before, after) != 1
            || !seen.insert((repair.source.clone(), before))
            || !families
                .npc_placements
                .iter()
                .any(|p| p.npc == repair.npc && p.cell == before)
            || families.npc_placements.iter().any(|p| p.cell == after)
            || spawns
                .iter()
                .flat_map(|s| &s.points)
                .any(|p| p.cell == after)
        {
            return Err(invalid("duplicate, floor, distance or occupied target"));
        }
        let creature = families
            .creatures
            .get(&repair.before.creature)
            .ok_or_else(|| invalid("missing creature"))?;
        if creature.boss || creature.encounter_bound {
            return Err(invalid("boss and encounter anchors cannot be repaired"));
        }
        let direction = match repair.before.direction.as_str() {
            "north" => Direction::North,
            "east" => Direction::East,
            "south" => Direction::South,
            "west" => Direction::West,
            _ => return Err(invalid("direction")),
        };
        let mut sources = spawns.iter_mut().filter(|s| s.key == repair.source);
        let source = sources.next().ok_or_else(|| invalid("missing source"))?;
        if sources.next().is_some() || source.centre != centre {
            return Err(invalid("source centre drifted"));
        }
        let radius = source
            .points
            .iter()
            .map(|p| distance(p.cell, centre))
            .max()
            .unwrap_or(0);
        if distance(after, centre) > radius {
            return Err(invalid("target outside captured source area"));
        }
        let mut points = source.points.iter_mut().filter(|p| {
            p.cell == before
                && p.creature == repair.before.creature
                && p.direction == direction
                && p.respawn_ms == repair.before.respawn_ms
                && p.boss_policy.is_none()
        });
        let point = points
            .next()
            .ok_or_else(|| invalid("exact before-state drifted"))?;
        if points.next().is_some() {
            return Err(invalid("ambiguous before-state"));
        }
        point.cell = after;
    }
    families.spawns = spawns;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{BuildClass, Identity};
    use crate::compile::{Input, realize_npcs, realize_spawns};
    use crate::project::{CreatureFacts, NpcPlacement, SpawnPoint, SpawnSource};

    fn document() -> Result<Document, Error> {
        serde_json::from_slice(include_bytes!(
            "../../../../content/npcs/spawn-anchor-repairs.json"
        ))
        .map_err(|error| invalid(&error.to_string()))
    }

    fn families() -> Families {
        let mut families = Families::default();
        families.creatures.insert(
            "oteryn:creature.bat".into(),
            CreatureFacts {
                period: "all".into(),
                boss: false,
                encounter_bound: false,
            },
        );
        families.spawns.push(SpawnSource {
            key: "oteryn:spawn.x32604_y31939_z10".into(),
            centre: (32604, 31939, 10),
            points: vec![SpawnPoint {
                creature: "oteryn:creature.bat".into(),
                cell: (32605, 31939, 10),
                direction: Direction::North,
                respawn_ms: 90000,
                boss_policy: None,
            }],
        });
        families.npc_placements.push(NpcPlacement {
            key: "oteryn:npc_placement.malech.x32605_y31939_z10".into(),
            npc: "oteryn:npc.malech".into(),
            cell: (32605, 31939, 10),
            direction: Direction::North,
        });
        families
    }

    #[test]
    fn correction_preserves_every_other_spawn_and_npc_fact() -> Result<(), Error> {
        let mut families = families();
        let before = families.spawns.clone();
        let npcs = families.npc_placements.clone();
        apply_document(document()?, &mut families)?;
        let mut expected = before;
        expected[0].points[0].cell = (32604, 31940, 10);
        assert_eq!(families.spawns, expected);
        assert_eq!(families.npc_placements, npcs);
        Ok(())
    }

    #[test]
    fn late_drift_rejects_the_whole_successor() -> Result<(), Error> {
        let mut value: Value = serde_json::from_slice(include_bytes!(
            "../../../../content/npcs/spawn-anchor-repairs.json"
        ))
        .map_err(|error| invalid(&error.to_string()))?;
        let second = value["repairs"][0].clone();
        value["repairs"]
            .as_array_mut()
            .ok_or_else(|| invalid("repairs"))?
            .push(second);
        let document =
            serde_json::from_value(value).map_err(|error| invalid(&error.to_string()))?;
        let mut families = families();
        let before = families.spawns.clone();
        assert!(apply_document(document, &mut families).is_err());
        assert_eq!(families.spawns, before);
        Ok(())
    }

    #[test]
    fn floor_distance_and_source_area_remain_bounded() -> Result<(), Error> {
        for after in [
            Cell {
                x: 32604,
                y: 31940,
                floor: 11,
            },
            Cell {
                x: 32606,
                y: 31939,
                floor: 10,
            },
            Cell {
                x: 32605,
                y: 31939,
                floor: 10,
            },
        ] {
            let mut document = document()?;
            document.repairs[0].after = after;
            let mut families = families();
            let before = families.spawns.clone();
            assert!(apply_document(document, &mut families).is_err());
            assert_eq!(families.spawns, before);
        }
        Ok(())
    }

    #[test]
    fn before_delay_and_boss_ownership_cannot_drift() -> Result<(), Error> {
        let mut changed = document()?;
        changed.repairs[0].before.respawn_ms += 1;
        assert!(apply_document(changed, &mut families()).is_err());
        let mut families = families();
        let creature = families
            .creatures
            .get_mut("oteryn:creature.bat")
            .ok_or_else(|| invalid("bat"))?;
        creature.boss = true;
        assert!(apply_document(document()?, &mut families).is_err());
        Ok(())
    }

    #[test]
    fn real_source_keeps_both_bat_and_stationary_malech_admitted() -> Result<(), Error> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let project = crate::source::project_regions(&root, &[(127, 124)])?;
        let (resolver, palette) = crate::source::registry(&root, &project)?;
        let input = Input {
            regions: &project.regions,
            palette: &palette,
            identity: Identity::default(),
            world: crate::source::world_extent(&root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &project.families,
        };
        let (spawns, _) = realize_spawns(&input, &resolver)?;
        let source = spawns
            .sources
            .iter()
            .find(|s| s.key == "oteryn:spawn.x32604_y31939_z10")
            .ok_or_else(|| invalid("bat source was dropped"))?;
        assert_eq!(source.floor, -10);
        assert_eq!(source.points.len(), 1);
        assert_eq!((source.points[0].x, source.points[0].y), (32604, 31940));
        let (npcs, report) = realize_npcs(&input, &resolver, &spawns)?;
        let npc = npcs
            .npcs
            .iter()
            .find(|n| n.key == "oteryn:npc.malech")
            .ok_or_else(|| invalid(&format!("Malech held: {:?}", report.held)))?;
        assert_eq!(npc.placements.len(), 1);
        assert_eq!(
            (
                npc.placements[0].x,
                npc.placements[0].y,
                npc.placements[0].floor
            ),
            (32605, 31939, -10)
        );
        Ok(())
    }
}
