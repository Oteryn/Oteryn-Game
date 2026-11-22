//! Keep the Dreamer room edits under the existing Channel callback projection lock.
use super::*;
use crate::map::facts::QuestCallbackProjection;
use crate::map::view::EntryFacts;
use oteryn_gameplay_quests::quest::dreamer_puzzles as puzzle;

type Pillow = (u64, u64, EntryFacts, u16);

fn pillow(map: &BundleMap, position: Position) -> Option<Pillow> {
    let pos = tile(position);
    if map.facts.house_tile(pos) {
        return None;
    }
    let base = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
    let mut matches = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, d)| **d == 0)
        .enumerate()
        .filter_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let key = placement_key(pos, ordinal)?;
            let original = map
                .facts
                .quest_callback_base_appearance(pos, ordinal, *id)?;
            let current = map.facts.base_entry(pos, ordinal, *id)?;
            (!map.facts.base_hidden(key)
                && puzzle::PILLOWS.contains(&original)
                && puzzle::PILLOWS.contains(&current.appearance_id))
            .then_some((key, map.facts.object_revision(key), current, original))
        });
    let entry = matches.next()?;
    matches.next().is_none().then_some(entry)
}

pub(super) fn matches(map: &BundleMap, callback: &Callback) -> bool {
    match callback.key.as_str() {
        puzzle::COLOUR_EXIT => (0..6).all(|y| {
            (0..6).all(|x| {
                pillow(map, puzzle::board_cell(x, y)).is_some_and(|(_, _, facts, _)| {
                    puzzle::solved_colour(x, y, facts.appearance_id)
                })
            })
        }),
        puzzle::WALL_EXIT => puzzle::WALL_CONTROLS.iter().all(|&(x, y)| {
            let pos = TilePos { x, y, floor: -14 };
            // The source walls can return a second before their levers. Check the
            // separately timed wall instances by their original source slots.
            let Some(callbacks) = WALLS
                .get_or_init(|| {
                    world_callbacks::load().ok().map(|registered| {
                        registered
                            .into_iter()
                            .filter(|c| {
                                c.key
                                    .starts_with("oteryn:quest-callback/m07/dreamer-lever-")
                            })
                            .collect()
                    })
                })
                .as_ref()
            else {
                return false;
            };
            callbacks.iter().any(|lever| {
                lever.selector.position.is_some_and(|p| tile(p) == pos)
                    && lever.passages.iter().all(|passage| {
                        let at = tile(passage.position);
                        let Some(base) = map.overlay.base().tile(at.x, at.y, at.floor) else {
                            return false;
                        };
                        base.ids()
                            .iter()
                            .zip(base.depths())
                            .filter(|(_, d)| **d == 0)
                            .enumerate()
                            .all(|(ordinal, (id, _))| {
                                let Ok(ordinal) = u8::try_from(ordinal) else {
                                    return false;
                                };
                                map.facts.quest_callback_base_appearance(at, ordinal, *id)
                                    != Some(passage.appearance)
                                    || placement_key(at, ordinal)
                                        .is_some_and(|key| map.facts.base_hidden(key))
                            })
                    })
            })
        }),
        _ => true,
    }
}

static WALLS: std::sync::OnceLock<Option<Vec<Callback>>> = std::sync::OnceLock::new();

pub(super) fn prepare(
    map: &BundleMap,
    callback: &Callback,
    source: u64,
    projection: Option<QuestCallbackProjection>,
) -> Result<Option<QuestCallbackProjection>, UseDisposition> {
    let Some((at, _)) = crate::map::view::placement_of(source) else {
        return Ok(projection);
    };
    let control = Position {
        x: at.x,
        y: at.y,
        floor: at.floor,
    };
    let (cells, next) = if callback.key == puzzle::COLOUR_EXIT {
        let cells: Vec<_> = (0..6)
            .flat_map(|y| (0..6).map(move |x| puzzle::board_cell(x, y)))
            .map(|position| pillow(map, position))
            .collect::<Option<_>>()
            .ok_or(UseDisposition::Rejected)?;
        let next: Vec<_> = cells.iter().map(|(_, _, _, original)| *original).collect();
        (cells, next)
    } else if let Some(line) = puzzle::shift_line(callback, control) {
        let cells: Vec<_> = line
            .into_iter()
            .map(|position| pillow(map, position))
            .collect::<Option<_>>()
            .ok_or(UseDisposition::Rejected)?;
        let mut next: Vec<_> = cells
            .iter()
            .map(|(_, _, facts, _)| facts.appearance_id)
            .collect();
        next.rotate_right(1);
        (cells, next)
    } else {
        return Ok(projection);
    };
    if projection.is_some() {
        return Err(UseDisposition::Rejected);
    }
    let mut entries = Vec::with_capacity(cells.len());
    for ((key, revision, before, _), appearance) in cells.into_iter().zip(next) {
        let replacement = if before.appearance_id == appearance {
            None
        } else {
            let mut replacement = map
                .facts
                .quest_callback_replacement(key, appearance)
                .ok_or(UseDisposition::Rejected)?;
            // This is one original configured puzzle slot, not a portable mint.
            replacement.pickupable = false;
            replacement.bound = true;
            Some(replacement)
        };
        entries.push((key, revision, replacement));
    }
    Ok(Some(QuestCallbackProjection::ReadsetTransforms {
        entries,
        reset: None,
    }))
}
