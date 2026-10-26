//! Owner option A: basic Oteryn rules, not Reference-qualified.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// Discovery rule parameters. Native area identity and geometry come from the admitted
// World bundle; donor geometry is never used as map authority.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasicArea {
    pub revision: String,
    pub qualification: String,
    pub area: u32,
    pub subarea: u32,
    pub name: String,
    pub radius: i32,
    pub pool_limit: usize,
    pub generated_count: usize,
    pub found_radius: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}
impl Point {
    pub fn in_window(self, at: Self) -> bool {
        // protocol-oteryn world_map: 18x14, left 8/top 6, same-floor POIs only.
        self.floor == at.floor
            && (-8..=9).contains(&(i64::from(self.x) - i64::from(at.x)))
            && (-6..=7).contains(&(i64::from(self.y) - i64::from(at.y)))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exploration {
    pub points: Vec<Point>,
    pub found: u8,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapProgress {
    pub discovered: bool,
    pub exploration: Option<Exploration>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapAction {
    Query,
    Start,
    Shuffle,
    Find,
    Donate(u32),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapRefusal {
    Invalid,
    PremiumRequired,
    AlreadyDiscovered,
    NoExploration,
    InsufficientPool,
}

/// Map information unlocked by the share of a main area's discovered subareas.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AreaDisclosure {
    pub npcs: bool,
    pub passages_and_teleports: bool,
    pub creatures: bool,
}

impl AreaDisclosure {
    /// Uses integer cross multiplication so 1/3 does not accidentally satisfy 30% due to
    /// rounding, while an exact threshold always does.
    pub fn for_progress(discovered: u32, total: u32) -> Option<Self> {
        if total == 0 || discovered > total {
            return None;
        }
        let discovered = u64::from(discovered);
        let total = u64::from(total);
        Some(Self {
            npcs: discovered * 100 >= total * 30,
            passages_and_teleports: discovered * 100 >= total * 70,
            creatures: discovered == total,
        })
    }
}

/// The next permanent reward Charos may grant. Grants at lower thresholds must already exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharosReward {
    Outfit,
    FirstAddon,
    SecondAddon,
}

pub fn next_charos_reward(completed_areas: u8, granted_stage: u8) -> Option<CharosReward> {
    match (completed_areas, granted_stage) {
        (20.., 2) => Some(CharosReward::SecondAddon),
        (15.., 1) => Some(CharosReward::FirstAddon),
        (10.., 0) => Some(CharosReward::Outfit),
        _ => None,
    }
}

/// Summer-2026 Discovery speed inside a fully discovered area.
///
/// The endpoints are official Global evidence. Owner decision 22 supplies the per-area fallback
/// associated with pinned Crystal `201189f871d23597b82caf88fd611454a2ddefb8`; it is deliberately
/// marked not verified against Global. Twenty discoverable areas therefore yield 100..=195.
pub fn discovery_speed_bonus(completed_areas: u8, current_area_discovered: bool) -> u16 {
    if !current_area_discovered || completed_areas == 0 {
        return 0;
    }
    100 + 5 * u16::from(completed_areas.min(20) - 1)
}

/// Select the daily donation and random improved-respawn areas from all eligible areas.
/// The highest qualifying pool wins (lowest id breaks an exact tie); the random lane excludes
/// both yesterday's improved areas and today's donation winner.
pub fn daily_area_boosts(
    areas: &[(u32, u64)],
    previous: &[u32],
    random_draw: u64,
) -> (Option<u32>, Option<u32>) {
    let donation = areas
        .iter()
        .filter(|(area, pool)| *pool >= 10_000_000 && !previous.contains(area))
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(area, _)| *area);
    let candidates: Vec<_> = areas
        .iter()
        .map(|(area, _)| *area)
        .filter(|area| Some(*area) != donation && !previous.contains(area))
        .collect();
    let random = (!candidates.is_empty() && random_draw % 100 < 33)
        .then(|| candidates[(random_draw as usize / 100) % candidates.len()]);
    (donation, random)
}
impl BasicArea {
    pub fn content_digest() -> [u8; 32] {
        Sha256::digest(include_bytes!(
            "../../../../../../content/cyclopedia/basic-area.json"
        ))
        .into()
    }

    pub fn embedded() -> Option<Self> {
        let value: Self = serde_json::from_str(include_str!(
            "../../../../../../content/cyclopedia/basic-area.json"
        ))
        .ok()?;
        (value.area == 1
            && value.subarea == 1
            && value.radius == 32
            && value.pool_limit == 128
            && value.generated_count == 7
            && value.found_radius == 0
            && value.revision == "oteryn-discovery-global-2026-v1"
            && value.qualification
                == "Global Summer 2026: automatic activation and seven POIs; native geometry from the pinned world bundle"
            && !value.name.is_empty())
        .then_some(value)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &self,
        state: &mut MapProgress,
        action: MapAction,
        pool: &[Point],
        at: Point,
        premium: bool,
        occurrence: &[u8; 16],
    ) -> Result<bool, MapRefusal> {
        if (state.discovered && state.exploration.is_some())
            || pool.iter().enumerate().any(|(i, p)| pool[..i].contains(p))
            || pool.len() > self.pool_limit
            || state.exploration.as_ref().is_some_and(|e| {
                e.points.len() != self.generated_count
                    || e.points
                        .iter()
                        .enumerate()
                        .any(|(i, p)| e.points[..i].contains(p))
                    || e.points.iter().any(|p| !pool.contains(p))
            })
        {
            return Err(MapRefusal::Invalid);
        }
        match action {
            MapAction::Donate(_) => Ok(false),
            // Summer 2026 automatically activates subareas. A query is the first
            // authenticated observation available to this owner, so it creates the
            // character-private seven-POI exploration without a Premium gate.
            MapAction::Query if state.discovered || state.exploration.is_some() => Ok(false),
            MapAction::Query => {
                let mut candidates: Vec<_> = pool
                    .iter()
                    .copied()
                    .filter(|p| !p.in_window(at))
                    .map(|p| {
                        let mut hash = Sha256::new();
                        hash.update(b"cyclopedia_poi:global-seven-v1");
                        hash.update(occurrence);
                        hash.update(p.x.to_be_bytes());
                        hash.update(p.y.to_be_bytes());
                        hash.update(p.floor.to_be_bytes());
                        (<[u8; 32]>::from(hash.finalize()), p)
                    })
                    .collect();
                candidates.sort_unstable();
                if candidates.len() < self.generated_count {
                    return Err(MapRefusal::InsufficientPool);
                }
                state.exploration = Some(Exploration {
                    points: candidates
                        .into_iter()
                        .take(self.generated_count)
                        .map(|(_, p)| p)
                        .collect(),
                    found: 0,
                });
                Ok(true)
            }
            MapAction::Start | MapAction::Shuffle => {
                if !premium {
                    return Err(MapRefusal::PremiumRequired);
                }
                if state.discovered {
                    return Err(MapRefusal::AlreadyDiscovered);
                }
                if action == MapAction::Shuffle && state.exploration.is_none() {
                    return Err(MapRefusal::NoExploration);
                }
                let mut candidates: Vec<_> = pool
                    .iter()
                    .copied()
                    .filter(|p| !p.in_window(at))
                    .map(|p| {
                        let mut hash = Sha256::new();
                        hash.update(b"cyclopedia_poi:oteryn-basic-v1");
                        hash.update(occurrence);
                        hash.update(p.x.to_be_bytes());
                        hash.update(p.y.to_be_bytes());
                        hash.update(p.floor.to_be_bytes());
                        (<[u8; 32]>::from(hash.finalize()), p)
                    })
                    .collect();
                candidates.sort_unstable();
                if candidates.len() < self.generated_count {
                    return Err(MapRefusal::InsufficientPool);
                }
                state.exploration = Some(Exploration {
                    points: candidates
                        .into_iter()
                        .take(self.generated_count)
                        .map(|(_, p)| p)
                        .collect(),
                    found: 0,
                });
                Ok(true)
            }
            MapAction::Find => {
                let activated = if state.exploration.is_none() && !state.discovered {
                    self.apply(state, MapAction::Query, pool, at, false, occurrence)?
                } else {
                    false
                };
                let Some(exploration) = state.exploration.as_mut() else {
                    return Err(MapRefusal::NoExploration);
                };
                let before = exploration.found;
                for (index, point) in exploration.points.iter().enumerate() {
                    if point.floor == at.floor
                        && point.x.abs_diff(at.x) <= self.found_radius as u32
                        && point.y.abs_diff(at.y) <= self.found_radius as u32
                    {
                        exploration.found |= 1 << index;
                    }
                }
                if exploration.found.count_ones() >= 7 {
                    state.discovered = true;
                    state.exploration = None;
                }
                Ok(activated
                    || state.discovered
                    || state
                        .exploration
                        .as_ref()
                        .is_some_and(|e| e.found != before))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    use super::*;
    #[test]
    fn disclosure_charos_and_local_speed_follow_area_progress() {
        assert_eq!(AreaDisclosure::for_progress(0, 0), None);
        assert_eq!(
            AreaDisclosure::for_progress(3, 10),
            Some(AreaDisclosure {
                npcs: true,
                passages_and_teleports: false,
                creatures: false,
            })
        );
        assert_eq!(
            AreaDisclosure::for_progress(7, 10),
            Some(AreaDisclosure {
                npcs: true,
                passages_and_teleports: true,
                creatures: false,
            })
        );
        assert_eq!(
            AreaDisclosure::for_progress(10, 10),
            Some(AreaDisclosure {
                npcs: true,
                passages_and_teleports: true,
                creatures: true,
            })
        );
        assert_eq!(next_charos_reward(9, 0), None);
        assert_eq!(next_charos_reward(10, 0), Some(CharosReward::Outfit));
        assert_eq!(next_charos_reward(15, 1), Some(CharosReward::FirstAddon));
        assert_eq!(next_charos_reward(20, 2), Some(CharosReward::SecondAddon));
        assert_eq!(next_charos_reward(20, 3), None);
        assert_eq!(discovery_speed_bonus(20, false), 0);
        assert_eq!(discovery_speed_bonus(0, true), 0);
        assert_eq!(discovery_speed_bonus(1, true), 100);
        assert_eq!(discovery_speed_bonus(10, true), 145);
        assert_eq!(discovery_speed_bonus(20, true), 195);
        assert_eq!(discovery_speed_bonus(21, true), 195);
        assert_eq!(
            daily_area_boosts(&[(23, 10_000_000), (24, 12_000_000), (25, 0)], &[], 0),
            (Some(24), Some(23))
        );
        assert_eq!(
            daily_area_boosts(&[(23, 12_000_000), (24, 12_000_000)], &[23], 32),
            (Some(24), None)
        );
    }
    #[test]
    fn cyclopedia_basic_progress_is_private_deterministic_and_seven_finds_complete() {
        let Some(area) = BasicArea::embedded() else {
            panic!("authored area")
        };
        let pool: Vec<_> = (20..40)
            .map(|x| Point {
                x,
                y: 10,
                floor: -7,
            })
            .collect();
        let start = Point {
            x: 0,
            y: 10,
            floor: -7,
        };
        let mut state = MapProgress::default();
        assert_eq!(
            area.apply(&mut state, MapAction::Query, &pool, start, false, &[1; 16]),
            Ok(true)
        );
        assert_eq!(state.exploration.as_ref().map(|e| e.points.len()), Some(7));
        state = MapProgress::default();
        assert_eq!(
            area.apply(&mut state, MapAction::Start, &pool, start, false, &[1; 16]),
            Err(MapRefusal::PremiumRequired)
        );
        assert_eq!(state, MapProgress::default());
        assert_eq!(
            area.apply(&mut state, MapAction::Shuffle, &pool, start, true, &[1; 16]),
            Err(MapRefusal::NoExploration)
        );
        assert_eq!(
            area.apply(&mut state, MapAction::Start, &pool, start, true, &[1; 16]),
            Ok(true)
        );
        let mut replay = MapProgress::default();
        assert_eq!(
            area.apply(&mut replay, MapAction::Start, &pool, start, true, &[1; 16]),
            Ok(true)
        );
        assert_eq!(state, replay);
        let Some(exploration) = state.exploration.clone() else {
            panic!("active exploration")
        };
        let point = exploration.points[0];
        let elsewhere = Point { floor: -6, ..point };
        assert_eq!(
            area.apply(
                &mut state,
                MapAction::Find,
                &pool,
                elsewhere,
                false,
                &[2; 16]
            ),
            Ok(false)
        );
        assert_eq!(
            area.apply(&mut state, MapAction::Find, &pool, point, false, &[2; 16]),
            Ok(true)
        );
        assert_eq!(
            area.apply(&mut state, MapAction::Find, &pool, point, false, &[3; 16]),
            Ok(false)
        );
        assert_eq!(
            area.apply(&mut state, MapAction::Shuffle, &pool, start, true, &[2; 16]),
            Ok(true)
        );
        let Some(exploration) = state.exploration.clone() else {
            panic!("shuffled exploration")
        };
        assert_eq!(exploration.found, 0);
        for p in exploration.points.iter().take(7) {
            assert_eq!(
                area.apply(&mut state, MapAction::Find, &pool, *p, false, &[3; 16]),
                Ok(true)
            );
        }
        assert!(state.discovered);
        assert!(state.exploration.is_none());
        assert_eq!(
            area.apply(&mut state, MapAction::Start, &pool, start, true, &[3; 16]),
            Err(MapRefusal::AlreadyDiscovered)
        );
        let mut empty = MapProgress::default();
        assert_eq!(
            area.apply(
                &mut empty,
                MapAction::Start,
                &pool,
                Point { x: 30, ..point },
                true,
                &[3; 16]
            ),
            Err(MapRefusal::InsufficientPool)
        );
    }
}
