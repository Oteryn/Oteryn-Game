//! Awash placement witnesses from WVAR 5fe12041 and Crystal 201189f8.
use super::super::boot::{BundleSpawnFacts, BundleWorld};
use super::super::overlay::TilePos;
use super::*;

const SCOUT: &str = "oteryn:creature.deepling_scout";
const GROAM: &str = "oteryn:creature.groam";
const ENTRANCE: (u16, u16, i8) = (32516, 32018, 9);

const DRY: &[PhaseTile] = &[
    PhaseTile {
        at: (32576, 32014, 10),
        flags: 8,
        entries: &[10113, 577, 6379],
        exit: None,
    },
    PhaseTile {
        at: (32560, 32017, 10),
        flags: 8,
        entries: &[499, 1855],
        exit: None,
    },
    PhaseTile {
        at: (32516, 32018, 10),
        flags: 8,
        entries: &[499],
        exit: None,
    },
    PhaseTile {
        at: (32515, 32018, 10),
        flags: 8,
        entries: &[499, 1949],
        exit: Some((32515, 32018, 9)),
    },
    PhaseTile {
        at: (32515, 32023, 10),
        flags: 8,
        entries: &[499, 6374, 1855],
        exit: None,
    },
    PhaseTile {
        at: (32541, 32018, 10),
        flags: 8,
        entries: &[351, 6384, 1856],
        exit: None,
    },
    PhaseTile {
        at: (32573, 32020, 10),
        flags: 8,
        entries: &[499],
        exit: None,
    },
    PhaseTile {
        at: (32512, 32026, 10),
        flags: 8,
        entries: &[499, 6379],
        exit: None,
    },
    PhaseTile {
        at: (32524, 32023, 10),
        flags: 8,
        entries: &[499],
        exit: None,
    },
    PhaseTile {
        at: (32557, 32024, 10),
        flags: 8,
        entries: &[499, 6385],
        exit: None,
    },
    PhaseTile {
        at: (32506, 32025, 10),
        flags: 8,
        entries: &[499, 6382],
        exit: None,
    },
    PhaseTile {
        at: (32573, 32028, 10),
        flags: 8,
        entries: &[499],
        exit: None,
    },
    PhaseTile {
        at: (32549, 32029, 10),
        flags: 8,
        entries: &[499, 6380],
        exit: None,
    },
    PhaseTile {
        at: (32558, 32007, 9),
        flags: 0,
        entries: &[352, 12666],
        exit: None,
    },
    PhaseTile {
        at: (32560, 32014, 9),
        flags: 0,
        entries: &[352, 12667],
        exit: None,
    },
    PhaseTile {
        at: (32567, 32024, 9),
        flags: 0,
        entries: &[351, 12666],
        exit: None,
    },
    PhaseTile {
        at: (32516, 32018, 9),
        flags: 0,
        entries: &[352, 1949],
        exit: Some((32516, 32018, 10)),
    },
    PhaseTile {
        at: (32515, 32018, 9),
        flags: 0,
        entries: &[352],
        exit: None,
    },
];

const FLOODED: &[PhaseTile] = &[
    PhaseTile {
        at: (32516, 32018, 9),
        flags: 0,
        entries: &[622],
        exit: None,
    },
    PhaseTile {
        at: (32558, 32007, 9),
        flags: 0,
        entries: &[352, 12666],
        exit: None,
    },
    PhaseTile {
        at: (32560, 32014, 9),
        flags: 0,
        entries: &[352, 12667],
        exit: None,
    },
    PhaseTile {
        at: (32567, 32024, 9),
        flags: 0,
        entries: &[351, 12666],
        exit: None,
    },
    PhaseTile {
        at: (32515, 32018, 10),
        flags: 8,
        entries: &[499, 1949],
        exit: Some((32515, 32018, 9)),
    },
    PhaseTile {
        at: (32515, 32018, 9),
        flags: 0,
        entries: &[352],
        exit: None,
    },
];

const HOMES: &[(u16, u16)] = &[
    (32576, 32014),
    (32560, 32017),
    (32516, 32018),
    (32515, 32023),
    (32541, 32018),
    (32573, 32020),
    (32512, 32026),
    (32524, 32023),
    (32557, 32024),
    (32506, 32025),
    (32573, 32028),
    (32549, 32029),
];

pub(super) fn tiles(manifest: &Manifest, phase: Option<bool>) -> &'static [PhaseTile] {
    let (x, y, _) = ENTRANCE;
    if x < manifest.world.min_x
        || x >= manifest.world.max_x
        || y < manifest.world.min_y
        || y >= manifest.world.max_y
        || !manifest.world.floors.contains(&-9)
        || !manifest.world.floors.contains(&-10)
    {
        return &[];
    }
    match phase {
        Some(true) => DRY,
        Some(false) => FLOODED,
        None => &[],
    }
}

fn creature_at(dry: bool, home: usize, roll: u64) -> Option<(&'static str, u64)> {
    if !dry || home >= HOMES.len() {
        return None;
    }
    // The spoiler reports a possible Groam appearance. Use one World-day
    // draw at a native Scout home, with a one-percent Oteryn selection.
    Some(if home == HOMES.len() - 1 && roll < 10 {
        (GROAM, u64::from(oteryn_world_bundle::spawn::MAX_RESPAWN_MS))
    } else {
        (SCOUT, 90_000)
    })
}

pub(in crate::map) fn project_spawns(
    world: &BundleWorld,
    daily: &DailyMap,
    profile: &impl Fn(&str) -> Option<(u64, bool)>,
    sources: &mut Vec<BundleSpawnFacts>,
) -> Option<()> {
    let Some(dry) = daily.phases.get("awash").copied() else {
        return Some(());
    };
    let (x, y, _) = ENTRANCE;
    if world.base().tile(x, y, -9).is_none() {
        return Some(());
    }
    for (points, _) in &mut *sources {
        points.retain(|&((x, y, z), _, ref creature, _, _)| {
            z != 10
                || creature != SCOUT
                || !HOMES
                    .iter()
                    .any(|&(a, b)| i32::from(a) == x && i32::from(b) == y)
        });
    }
    sources.retain(|(points, _)| !points.is_empty());
    let roll = daily.pick(x, y, "awash-groam", 1000);
    for (home, &(x, y)) in HOMES.iter().enumerate() {
        let Some((creature, respawn)) = creature_at(dry, home, roll) else {
            continue;
        };
        if !world.enterable(TilePos { x, y, floor: -10 }) {
            continue;
        }
        let (health, blockable) = profile(creature)?;
        sources.push((
            vec![(
                (i32::from(x), i32::from(y), 10),
                respawn,
                creature.into(),
                health,
                blockable,
            )],
            1_000,
        ));
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn awash_spawn_phase_and_groam_chance_boundaries() {
        let boss_home = HOMES.len() - 1;
        assert_eq!(creature_at(false, boss_home, 0), None);
        assert_eq!(creature_at(true, 0, 0), Some((SCOUT, 90_000)));
        assert_eq!(creature_at(true, boss_home, 9), Some((GROAM, 86_400_000)));
        assert_eq!(creature_at(true, boss_home, 10), Some((SCOUT, 90_000)));
        assert_eq!(creature_at(true, HOMES.len(), 0), None);
        assert!(
            FLOODED
                .iter()
                .any(|tile| tile.at == ENTRANCE && tile.exit.is_none())
        );
        assert!(
            DRY.iter()
                .any(|tile| tile.at == ENTRANCE && tile.exit.is_some())
        );
    }
}
