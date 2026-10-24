//! Pure ordinary and native cast geometry; tile permission comes from the dispatch owners.
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
