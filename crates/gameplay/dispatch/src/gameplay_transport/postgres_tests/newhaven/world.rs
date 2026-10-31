//! Load the compiler's real artifact with the repository pin. No generated test terrain.
use super::*;
use crate::map::overlay::TilePos;
use std::collections::{BTreeMap, VecDeque};

pub(super) fn load(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let artifact = std::env::var_os("OTERYN_NEWHAVEN_TEST_BUNDLE")
        .ok_or("NEW HAVEN requires the pin-check artifact in OTERYN_NEWHAVEN_TEST_BUNDLE")?;
    let bytes = std::fs::read(artifact)?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let pin: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("content/world/pins/oteryn.json"))?)?;
    let text = |field: &str| -> TestResult<String> {
        Ok(pin[field].as_str().ok_or("World pin field")?.into())
    };
    let hex = text("digest")?;
    if hex.len() != 64 {
        return Err("World pin digest length".into());
    }
    let mut digest = [0; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)?;
    }
    assert_eq!(pin["entry_start"]["x"].as_i64(), Some(32534));
    assert_eq!(pin["entry_start"]["y"].as_i64(), Some(32513));
    assert_eq!(pin["entry_start"]["floor"].as_i64(), Some(-7));
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            bundle: crate::map::BundlePins {
                digest,
                project_format_version: text("project_format_version")?,
                world_schema_version: text("world_schema_version")?,
                content_revision: text("content_revision")?,
                production: pin["production"].as_bool().ok_or("production flag")?,
            },
            map_revision: format!("sha256:{hex}"),
            start: TilePos {
                x: 32534,
                y: 32513,
                floor: -7,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| crate::map::boot::native_item_definition(Some(native), key),
    )
    .map_err(debug)
}

/// Bounded orthogonal search over the actual World. The dispatch still validates every step,
/// floor transition, actor and pacing gate; this function supplies only movement intents.
pub(super) fn path(
    world: &crate::map::boot::BundleWorld,
    from: MovementLocalPosition,
    to: (i32, i32),
) -> TestResult<Vec<StepDirection>> {
    let start = (from.x, from.y);
    let mut previous = BTreeMap::from([(start, (start, StepDirection::North))]);
    let mut queue = VecDeque::from([start]);
    while let Some(at) = queue.pop_front() {
        if at == to {
            let mut steps = Vec::new();
            let mut cell = to;
            while cell != start {
                let (parent, direction) = previous[&cell];
                steps.push(direction);
                cell = parent;
            }
            steps.reverse();
            return Ok(steps);
        }
        for (dx, dy, direction) in [
            (0, -1, StepDirection::North),
            (1, 0, StepDirection::East),
            (0, 1, StepDirection::South),
            (-1, 0, StepDirection::West),
        ] {
            let next = (at.0 + dx, at.1 + dy);
            if !(32506..=32634).contains(&next.0) || !(32405..=32522).contains(&next.1)
                || previous.contains_key(&next)
                // Never route through the ferry door before exercising its USE owner.
                || [(32556, 32497), (32556, 32498)].contains(&next)
            {
                continue;
            }
            let tile = TilePos {
                x: u16::try_from(next.0)?,
                y: u16::try_from(next.1)?,
                floor: -7,
            };
            if world.enterable(tile) {
                previous.insert(next, (at, direction));
                queue.push_back(next);
            }
        }
    }
    Err(format!("NEW HAVEN real World has no route from {start:?} to {to:?}").into())
}

pub(super) fn anna_placement() -> TestResult<crate::foundation::NpcPlacementTable> {
    let artifact = std::env::var_os("OTERYN_NEWHAVEN_TEST_BUNDLE").ok_or("World artifact")?;
    let bytes = std::fs::read(artifact)?;
    let visited = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(())).map_err(debug)?;
    let anna = visited
        .npcs
        .npcs
        .into_iter()
        .find(|npc| npc.key == "oteryn:npc.anna")
        .ok_or("real bundle Anna placement")?;
    assert_eq!(anna.placements.len(), 1);
    let placements = anna
        .placements
        .into_iter()
        .map(|placement| {
            use oteryn_world_bundle::npc::Direction;
            let direction = match placement.direction {
                Direction::North => crate::foundation::MovementFacing::North,
                Direction::East => crate::foundation::MovementFacing::East,
                Direction::South => crate::foundation::MovementFacing::South,
                Direction::West => crate::foundation::MovementFacing::West,
            };
            crate::foundation::NpcPlacement {
                x: placement.x,
                y: placement.y,
                floor: placement.floor,
                direction,
            }
        })
        .collect();
    Ok(crate::foundation::NpcPlacementTable {
        npcs: vec![crate::foundation::NpcPlacements {
            key: anna.key,
            placements,
        }],
    })
}
