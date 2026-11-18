//! Crystal 201189f dreamers_challenge_quest/movement_riddle.lua:3-38 and
//! movement_riddle_teleport.lua:11-58. One Channel read set owns the shared board.
use super::*;
use crate::map::{
    facts::QuestCallbackProjection,
    view::{EntryFacts, EntrySource, compose_tile},
};
use oteryn_gameplay_quests::quest::world_callbacks::{ColorGrid, GridAxis};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
struct Cell {
    key: u64,
    revision: u64,
    entry: EntryFacts,
}

fn read(map: &BundleMap, position: TilePos) -> Option<Cell> {
    if map.facts.house_tile(position) {
        return None;
    }
    let facts = map.facts.pinned();
    let stack = compose_tile(&map.overlay, &facts, position).ok()??;
    // Crystal addresses stack index 1 (the first item after the ground).
    let entry = stack.entries.get(1)?;
    let EntrySource::Base {
        placement_key: key, ..
    } = entry.source
    else {
        return None;
    };
    // A portable instance stays under the Ground/custody owner.
    (!entry.facts.pickupable && entry.facts.count == 1).then(|| Cell {
        key,
        entry: entry.facts,
        revision: facts.object_revision(key),
    })
}

fn replacement(
    map: &BundleMap,
    cell: Cell,
    appearance: u16,
) -> Option<(u64, u64, Option<EntryFacts>)> {
    let after = if cell.entry.appearance_id == appearance {
        None
    } else {
        Some(
            map.facts
                .quest_callback_replacement(cell.key, appearance)
                .filter(|facts| !facts.pickupable && facts.count == 1)?,
        )
    };
    Some((cell.key, cell.revision, after))
}

pub(super) fn prepare(
    map: &BundleMap,
    source: TilePos,
    grid: &ColorGrid,
    command: CommandRef,
) -> Result<QuestCallbackProjection, UseDisposition> {
    let refused = UseDisposition::Rejected;
    let (cells, new_colors) = match grid {
        ColorGrid::Shift {
            axis,
            offset,
            length,
            colors,
        } => {
            let cells = (0..*length)
                .map(|index| {
                    let mut at = source;
                    let distance = u16::from(*offset) + u16::from(index);
                    let coordinate = match axis {
                        GridAxis::X => &mut at.x,
                        GridAxis::Y => &mut at.y,
                    };
                    *coordinate = coordinate.checked_add(distance)?;
                    let cell = read(map, at)?;
                    colors.contains(&cell.entry.appearance_id).then_some(cell)
                })
                .collect::<Option<Vec<_>>>()
                .ok_or(refused)?;
            let mut appearances = cells
                .iter()
                .map(|cell| cell.entry.appearance_id)
                .collect::<Vec<_>>();
            appearances.rotate_right(1);
            (cells, appearances)
        }
        ColorGrid::Reset { groups } => {
            let mut cells = Vec::new();
            let mut appearances = Vec::new();
            for group in groups {
                let start = cells.len();
                for position in &group.cells {
                    cells.push(read(map, tile(*position)).ok_or(refused)?);
                }
                let observed = cells.get(start).ok_or(refused)?.entry.appearance_id;
                if !group.colors.contains(&observed)
                    || cells[start..]
                        .iter()
                        .any(|cell| cell.entry.appearance_id != observed)
                {
                    return Err(refused);
                }
                for _ in &group.cells {
                    appearances.push(*group.colors.first().ok_or(refused)?);
                }
            }
            // The donor resets by randomly placing nine pillows of each color.
            // Hash a separately indexed draw for each entry of the exact multiset.
            let seed = Sha256::new()
                .chain_update(b"oteryn:quest-color-grid:v1")
                .chain_update(command.game_session_id().as_bytes())
                .chain_update(command.command_id().get().to_be_bytes())
                .finalize();
            let mut draws = (0u8..)
                .zip(appearances)
                .map(|(index, color)| {
                    let draw = Sha256::new()
                        .chain_update(&seed)
                        .chain_update([index])
                        .finalize();
                    (draw, color)
                })
                .collect::<Vec<_>>();
            draws.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            (cells, draws.into_iter().map(|(_, color)| color).collect())
        }
    };
    let entries = cells
        .into_iter()
        .zip(new_colors)
        .map(|(cell, color)| replacement(map, cell, color))
        .collect::<Option<Vec<_>>>()
        .ok_or(refused)?;
    Ok(QuestCallbackProjection::ReadsetTransforms {
        entries,
        reset: None,
    })
}
