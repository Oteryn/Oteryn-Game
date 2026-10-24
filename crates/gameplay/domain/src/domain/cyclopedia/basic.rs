//! Owner option A: basic Oteryn rules, not Reference-qualified.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// Oteryn defaults, not Reference-qualified: one authored area/subarea, eight
// positions, exact-tile finding, no speed contribution, donation before random draw.
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
            && value.generated_count == 8
            && value.found_radius == 0
            && value.revision == "oteryn-cyclopedia-basic-v1"
            && value.qualification == "Oteryn default, not Reference-qualified"
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
            MapAction::Query | MapAction::Donate(_) => Ok(false),
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
                Ok(state.discovered
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
