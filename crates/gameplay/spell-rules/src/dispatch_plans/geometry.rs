//! Pure ordinary and native cast geometry; tile permission comes from the dispatch owners.
use oteryn_gameplay_combat_rules::chain::{CHAIN_PATH_SEARCH_TILES, ChainShape, ChainSpec};
use oteryn_gameplay_content::content::spell_data::executable_catalog::AbilityProfile;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Spatial value independent of the runtime's actor and movement carriers.
pub type TilePosition = (i32, i32, i16);
/// Unit facing delta, including diagonals.
pub type Direction = (i32, i32);

/// Invalid or unavailable planning input; dispatch maps this to its protocol disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanRejected;

pub fn offset(p: TilePosition, dx: i32, dy: i32) -> Result<TilePosition, PlanRejected> {
    Ok((
        p.0.checked_add(dx).ok_or(PlanRejected)?,
        p.1.checked_add(dy).ok_or(PlanRejected)?,
        p.2,
    ))
}
pub fn facing_to(from: TilePosition, to: TilePosition) -> Direction {
    match (
        (i64::from(to.0) - i64::from(from.0)).signum(),
        (i64::from(to.1) - i64::from(from.1)).signum(),
    ) {
        (0, -1) => (0, -1),
        (1, -1) => (1, -1),
        (1, 0) => (1, 0),
        (1, 1) => (1, 1),
        (0, 1) => (0, 1),
        (-1, 1) => (-1, 1),
        (-1, 0) => (-1, 0),
        _ => (-1, -1),
    }
}
/// WU26-14 (Winter Update 2026 test server): the cardinal direction of a wave or beam aimed from
/// `from` at `to`. `|dx| >= |dy|` picks east or west, otherwise north or south, so east and west
/// win a tie. `None` when both name the same column and row.
pub fn aimed_cardinal(from: TilePosition, to: TilePosition) -> Option<Direction> {
    let dx = i64::from(to.0) - i64::from(from.0);
    let dy = i64::from(to.1) - i64::from(from.1);
    match (dx.signum(), dy.signum()) {
        (0, 0) => None,
        (x, _) if dx.abs() >= dy.abs() => Some((if x > 0 { 1 } else { -1 }, 0)),
        (_, y) => Some((0, if y > 0 { 1 } else { -1 })),
    }
}
/// Source players start facing south before a recorded turn or step.
pub const DEFAULT_PLAYER_SPELL_DIRECTION: Direction = (0, 1);

/// An explicit aim takes precedence over the recorded and initial headings.
pub fn player_spell_direction(aim: Option<Direction>, facing: Option<Direction>) -> Direction {
    aim.or(facing).unwrap_or(DEFAULT_PLAYER_SPELL_DIRECTION)
}

/// The adjacent tile used when a target-or-direction spell falls back to heading.
pub fn player_spell_front(
    caster: TilePosition,
    facing: Option<Direction>,
) -> Result<TilePosition, PlanRejected> {
    let (dx, dy) = player_spell_direction(None, facing);
    offset(caster, dx, dy)
}

#[cfg(test)]
mod player_facing_tests {
    use super::*;

    #[test]
    fn directional_cast_uses_initial_south_heading_until_the_first_turn() {
        assert_eq!(player_spell_direction(None, None), (0, 1));
        assert_eq!(player_spell_direction(None, Some((-1, 0))), (-1, 0));
        assert_eq!(player_spell_direction(Some((1, 0)), Some((-1, 0))), (1, 0));
    }

    #[test]
    fn target_or_direction_fallback_reads_the_tile_in_front_of_the_player() {
        assert_eq!(player_spell_front((10, 10, 7), None), Ok((10, 11, 7)));
        assert_eq!(
            player_spell_front((10, 10, 7), Some((-1, 0))),
            Ok((9, 10, 7))
        );
        assert_eq!(
            player_spell_front((10, i32::MAX, 7), None),
            Err(PlanRejected)
        );
    }
}

/// SPELL-TARGET-1: the geometry origin and facing of a held-target cast. A directional or aimed
/// spell keeps the caster as origin (the tile in front of it) and takes only its facing from the
/// target; a single-target spell is centred on the target tile.
pub fn attack_target_origin(
    caster: TilePosition,
    target: TilePosition,
    directional: bool,
) -> Result<(TilePosition, Option<Direction>), PlanRejected> {
    if directional && target != caster {
        let facing = facing_to(caster, target);
        let (dx, dy) = facing;
        Ok((offset(caster, dx, dy)?, Some(facing)))
    } else {
        Ok((target, None))
    }
}
fn add_chain_path_tiles(
    tiles: &mut BTreeSet<TilePosition>,
    from: TilePosition,
) -> Result<(), PlanRejected> {
    let radius = i32::try_from(CHAIN_PATH_SEARCH_TILES).map_err(|_| PlanRejected)?;
    for dx in -radius..=radius {
        for dy in -radius..=radius {
            tiles.insert(offset(from, dx, dy)?);
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
pub struct ChainSearch {
    pub tiles: BTreeSet<TilePosition>,
    pub sight: BTreeSet<(TilePosition, TilePosition)>,
    pub visual_tiles: BTreeSet<TilePosition>,
}

/// Crystal 201189f8 combat.cpp:1881 searches around the current bounce, and
/// :1329–1344 searches for the visual path only from the origin of a jump.
pub fn ordinary_chain_search(
    spec: Option<&ChainSpec>,
    caster: TilePosition,
    explicit: Option<(TilePosition, u32)>,
    targets: BTreeSet<TilePosition>,
) -> Result<ChainSearch, PlanRejected> {
    let Some(spec) = spec else {
        return Ok(ChainSearch::default());
    };
    let within = |a: TilePosition, b: TilePosition, radius| {
        a.2 == b.2 && a.0.abs_diff(b.0) <= radius && a.1.abs_diff(b.1) <= radius
    };
    let mut frontier = targets
        .iter()
        .copied()
        .filter(|&p| match explicit {
            Some((target, range)) => p == target && within(caster, p, range),
            None => within(caster, p, spec.initial_range_tiles),
        })
        .collect::<BTreeSet<_>>();
    let mut result = ChainSearch {
        tiles: frontier.clone(),
        sight: frontier.iter().map(|&to| (caster, to)).collect(),
        visual_tiles: BTreeSet::new(),
    };
    let depth = match spec.shape {
        ChainShape::Sequential => spec.max_targets,
        ChainShape::Fork => spec.max_targets.min(1),
    };
    let mut expanded = BTreeSet::new();
    for _ in 0..depth {
        if frontier.is_empty() {
            break;
        }
        let mut next = BTreeSet::new();
        for from in frontier {
            if !expanded.insert(from) {
                continue;
            }
            for &to in &targets {
                if from != to && within(from, to, spec.range_tiles) {
                    result.sight.insert((from, to));
                    result.tiles.insert(to);
                    if !expanded.contains(&to) {
                        next.insert(to);
                    }
                }
            }
        }
        frontier = next;
    }
    if spec.asset_binding.is_some() {
        let origins = result
            .sight
            .iter()
            .map(|&(from, _)| from)
            .collect::<BTreeSet<_>>();
        for from in origins {
            add_chain_path_tiles(&mut result.visual_tiles, from)?;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod chain_search_tests {
    use super::*;

    #[test]
    fn a_non_chain_cast_does_not_require_spectator_tiles() -> Result<(), PlanRejected> {
        let caster = (32357, 32196, 7);
        let target = (32357, 32197, 7);
        let unrelated = (32356, 32196, 7);
        let mut cast_tiles = BTreeSet::from([caster, target]);
        let search = ordinary_chain_search(None, caster, None, BTreeSet::from([unrelated]))?;

        cast_tiles.extend(search.tiles);
        assert_eq!(cast_tiles, BTreeSet::from([caster, target]));
        assert!(search.sight.is_empty());
        Ok(())
    }

    #[test]
    fn optional_chain_visual_tiles_are_not_required_hit_geometry() -> Result<(), PlanRejected> {
        let caster = (32369, 32230, 7);
        let first = (32369, 32229, 7);
        let missing_visual = (32360, 32235, 7);
        let spec = ChainSpec {
            max_targets: 4,
            range_tiles: 7,
            initial_range_tiles: 7,
            shape: oteryn_gameplay_combat_rules::chain::ChainShape::Sequential,
            damage_step_percent: -5,
            asset_binding: Some("chain".into()),
        };
        let search = ordinary_chain_search(Some(&spec), caster, None, BTreeSet::from([first]))?;
        assert_eq!(search.sight, BTreeSet::from([(caster, first)]));
        assert_eq!(search.tiles, BTreeSet::from([first]));
        assert!(!search.tiles.contains(&missing_visual));
        assert!(search.visual_tiles.contains(&missing_visual));
        Ok(())
    }

    #[test]
    fn ordinary_chain_search_keeps_only_the_authored_bounces_and_visual_paths()
    -> Result<(), PlanRejected> {
        let spec = ChainSpec {
            max_targets: 3,
            range_tiles: 7,
            initial_range_tiles: 7,
            shape: oteryn_gameplay_combat_rules::chain::ChainShape::Fork,
            damage_step_percent: 0,
            asset_binding: Some(String::from("chain")),
        };
        let caster = (100, 100, 7);
        let first = (107, 100, 7);
        let bounce = (114, 100, 7);
        let beyond_fork = (121, 100, 7);
        let spectator = (135, 140, 7);
        let search = ordinary_chain_search(
            Some(&spec),
            caster,
            Some((first, 7)),
            BTreeSet::from([first, bounce, beyond_fork, spectator]),
        )?;
        assert_eq!(
            search.sight,
            BTreeSet::from([(caster, first), (first, bounce)])
        );
        assert!(search.visual_tiles.contains(&(91, 100, 7)));
        assert!(search.visual_tiles.contains(&(116, 100, 7)));
        assert!(!search.tiles.contains(&beyond_fork));
        assert!(!search.tiles.contains(&spectator));
        assert!(!search.visual_tiles.contains(&beyond_fork));
        assert!(!search.visual_tiles.contains(&spectator));
        Ok(())
    }
}

/// Add native target positions that can be reached outside the authored area.
pub fn extend_native_target_tiles<T: Ord>(
    tiles: &mut BTreeSet<T>,
    target_positions: impl IntoIterator<Item = T>,
    has_chain: bool,
) {
    if has_chain {
        tiles.extend(target_positions);
    }
}

#[cfg(test)]
mod native_target_tile_tests {
    use super::*;

    #[test]
    fn native_area_casts_do_not_read_unrelated_occupied_tiles() {
        let caster = (32369, 32230, 7);
        let area_tile = (32368, 32230, 7);
        let spectator = (32356, 32196, 7);
        let mut tiles = BTreeSet::from([caster, area_tile]);

        extend_native_target_tiles(&mut tiles, [spectator], false);
        assert_eq!(tiles, BTreeSet::from([caster, area_tile]));

        extend_native_target_tiles(&mut tiles, [spectator], true);
        assert!(tiles.contains(&spectator));
    }
}

pub fn geometry(
    a: &AbilityProfile,
    origin: TilePosition,
    facing: Direction,
) -> Result<BTreeSet<TilePosition>, PlanRejected> {
    let Some(area) = &a.area else {
        return Ok(BTreeSet::from([origin]));
    };
    let matrix = |rows: &[String]| -> Result<Value, PlanRejected> {
        rows.iter()
            .map(|r| {
                r.bytes()
                    .map(|c| match c {
                        b'.' => Ok(json!(0)),
                        b'x' => Ok(json!(1)),
                        b'c' => Ok(json!(2)),
                        b'C' => Ok(json!(3)),
                        _ => Err(PlanRejected),
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::Array)
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array)
    };
    let mut area_json =
        json!({"directional":a.needs_direction,"orthogonal":matrix(&area.matrix.north)?});
    if let Some(rows) = &area.matrix.diagonal {
        area_json["diagonal"] = matrix(rows)?;
    }
    area_candidates(&json!({"area":area_json}), origin, facing)
}
/// Geometry is read from the already closed native recipe, independent of tile permission.
pub fn area_candidates(
    parameters: &Value,
    origin: TilePosition,
    d: Direction,
) -> Result<BTreeSet<TilePosition>, PlanRejected> {
    let mut out = BTreeSet::new();
    let Some(area) = parameters.get("area") else {
        return Ok(out);
    };
    let directional = area["directional"].as_bool().ok_or(PlanRejected)?;
    let diagonal = area["diagonal"].is_array();
    let (matrix, turn) = if !directional {
        (&area["orthogonal"], 0)
    } else {
        match d {
            (-1, -1) if diagonal => (&area["diagonal"], 0),
            (1, -1) if diagonal => (&area["diagonal"], 1),
            (1, 1) if diagonal => (&area["diagonal"], 2),
            (-1, 1) if diagonal => (&area["diagonal"], 3),
            (0, -1) => (&area["orthogonal"], 0),
            (0, 1) => (&area["orthogonal"], 2),
            (-1, 0) | (-1, -1) | (-1, 1) => (&area["orthogonal"], 3),
            _ => (&area["orthogonal"], 1),
        }
    };
    let rows = matrix.as_array().ok_or(PlanRejected)?;
    let mut center = None;
    let mut active = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        for (x, v) in row.as_array().ok_or(PlanRejected)?.iter().enumerate() {
            let n = v.as_u64().ok_or(PlanRejected)?;
            if n >= 2 && center.replace((x as i32, y as i32)).is_some() {
                return Err(PlanRejected);
            }
            if n == 1 || n == 3 {
                active.push((x as i32, y as i32));
            }
        }
    }
    let (cx, cy) = center.ok_or(PlanRejected)?;
    for (x, y) in active {
        let (dx, dy) = (x - cx, y - cy);
        let (dx, dy) = match turn {
            0 => (dx, dy),
            1 => (-dy, dx),
            2 => (-dx, -dy),
            _ => (dy, -dx),
        };
        out.insert(offset(origin, dx, dy)?);
    }
    // Read the full side footprint even at stage0. It is not granted to the plan unless unlocked.
    if parameters.get("beam_mastery").is_some() {
        let central = out.clone();
        for p in central {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                out.insert(offset(p, dx, dy)?);
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileFlags {
    pub projectile: bool,
    pub solid: bool,
    pub floor_change: bool,
    pub protection: bool,
    pub present: bool,
}

/// Actual PvE permission only. PvP requires its own independently owned policy.
pub fn target_legal(
    t: &TileFlags,
    caster_tile: &TileFlags,
    health: u32,
    aggressive: bool,
    masterless_monster: bool,
    attackable: bool,
) -> bool {
    t.present
        && !t.floor_change
        && !t.solid
        && health > 0
        && (!aggressive || (!t.protection && !caster_tile.protection))
        && (!aggressive || (masterless_monster && attackable))
}

/// Qualified tile facts drive the bounded chain walking path.
pub fn path(
    from: TilePosition,
    to: TilePosition,
    search_tiles: u32,
    mut tile: impl FnMut(TilePosition) -> Option<TileFlags>,
) -> Option<Vec<TilePosition>> {
    use std::collections::VecDeque;
    if from.2 != to.2 {
        return None;
    }
    let mut queue = VecDeque::from([from]);
    let mut seen = BTreeMap::<TilePosition, Option<TilePosition>>::from([(from, None)]);
    while let Some(p) = queue.pop_front() {
        if p.0.abs_diff(to.0) <= 1 && p.1.abs_diff(to.1) <= 1 {
            let mut path = Vec::new();
            let mut cur = p;
            while let Some(Some(previous)) = seen.get(&cur) {
                path.push(cur);
                cur = *previous;
            }
            path.reverse();
            return Some(path);
        }
        for (dx, dy) in [
            (0, -1),
            (1, 0),
            (0, 1),
            (-1, 0),
            (-1, -1),
            (1, -1),
            (1, 1),
            (-1, 1),
        ] {
            let Ok(next) = offset(p, dx, dy) else {
                continue;
            };
            if next.0.abs_diff(from.0) > search_tiles
                || next.1.abs_diff(from.1) > search_tiles
                || seen.contains_key(&next)
                || !tile(next).is_some_and(|t| t.present && !t.solid && !t.floor_change)
            {
                continue;
            }
            if dx != 0
                && dy != 0
                && [offset(p, dx, 0).ok(), offset(p, 0, dy).ok()]
                    .into_iter()
                    .any(|side| {
                        side.and_then(&mut tile)
                            .is_none_or(|t| !t.present || t.solid)
                    })
            {
                continue;
            }
            seen.insert(next, Some(p));
            queue.push_back(next);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    fn p(x: i32, y: i32) -> TilePosition {
        (x, y, 7)
    }
    #[test]
    fn source_area_rotation_and_origin_are_not_permission() {
        let recipe = json!({"area":{"orthogonal":[[1],[2]],"diagonal":null,"directional":true}});
        assert_eq!(
            area_candidates(&recipe, p(9, 9), (0, -1)).unwrap(),
            BTreeSet::from([p(9, 8)])
        );
        assert_eq!(
            area_candidates(&recipe, p(9, 9), (1, 0)).unwrap(),
            BTreeSet::from([p(10, 9)])
        );
    }

    #[test]
    fn chain_path_requires_qualified_tiles_and_does_not_cut_blocked_corners() {
        let open = TileFlags {
            projectile: false,
            solid: false,
            floor_change: false,
            protection: false,
            present: true,
        };
        let tiles = BTreeMap::from([(p(1, 1), open), (p(2, 2), open)]);
        assert!(path(p(0, 0), p(3, 3), 9, |p| tiles.get(&p).copied()).is_none());
        let tiles = (0..=2)
            .flat_map(|x| (0..=2).map(move |y| (p(x, y), open)))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            path(p(0, 0), p(3, 3), 9, |p| tiles.get(&p).copied()),
            Some(vec![p(1, 1), p(2, 2)])
        );
        assert!(path(p(0, 0), p(3, 3), 1, |p| tiles.get(&p).copied()).is_none());
        assert!(path(p(0, 0), (3, 3, 8), 9, |p| tiles.get(&p).copied()).is_none());
    }

    #[test]
    fn aggressive_target_permission_requires_alive_attackable_masterless_pve() {
        let open = TileFlags {
            projectile: false,
            solid: false,
            floor_change: false,
            protection: false,
            present: true,
        };
        assert!(target_legal(&open, &open, 1, true, true, true));
        assert!(!target_legal(&open, &open, 0, false, false, false));
        assert!(!target_legal(&open, &open, 1, true, false, true));
        assert!(!target_legal(&open, &open, 1, true, true, false));
        let protected = TileFlags {
            protection: true,
            ..open
        };
        assert!(!target_legal(&protected, &open, 1, true, true, true));
        assert!(!target_legal(&open, &protected, 1, true, true, true));
        assert!(target_legal(&protected, &protected, 1, false, false, false));
        for invalid in [
            TileFlags {
                present: false,
                ..open
            },
            TileFlags {
                solid: true,
                ..open
            },
            TileFlags {
                floor_change: true,
                ..open
            },
        ] {
            assert!(!target_legal(&invalid, &open, 1, false, true, true));
        }
    }
}
