//! SUBAREA-ADMIT-1: immutable native membership, carried by the pinned World bundle.
use serde::{Deserialize, Serialize};

use crate::{Error, bundle::Extent};

pub const MAX_AREAS: usize = 1024;
pub const MAX_MASKS: usize = 512;
pub const MAX_RUNS: usize = 262_144;
pub const MAX_TILES: usize = 4_194_304;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Area {
    /// Client 15.30 cipsoft/map_area source id, never a compact content ordinal.
    pub id: u32,
    pub key: String,
    pub name: String,
    pub parent: Option<u32>,
    pub source_record_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mask {
    pub subarea: u32,
    pub asset: String,
    pub compressed_sha256: String,
    pub bmp_sha256: String,
    pub source_record_sha256: String,
    pub source_tiles: u32,
    pub admitted_tiles: u32,
}

/// Half-open horizontal run. Native floors are the negation of client z.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub floor: i8,
    pub y: u16,
    pub begin: u16,
    pub end: u16,
    pub subarea: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
    pub client_version: String,
    pub candidate_sha256: String,
    pub areas: Vec<Area>,
    pub masks: Vec<Mask>,
    /// Sorted by (floor, y, begin); disjoint, including across subareas.
    pub runs: Vec<Run>,
}

impl Table {
    /// The region and subarea at a native position. Missing coverage stays unknown.
    pub fn resolve(&self, x: u16, y: u16, floor: i8) -> Option<(u32, u32)> {
        let at = self
            .runs
            .partition_point(|r| (r.floor, r.y, r.begin) <= (floor, y, x));
        let run = self.runs.get(at.checked_sub(1)?)?;
        if run.floor != floor || run.y != y || x >= run.end {
            return None;
        }
        let area = &self.areas[self
            .areas
            .binary_search_by_key(&run.subarea, |a| a.id)
            .ok()?];
        Some((area.parent?, area.id))
    }

    pub(crate) fn validate(&self, world: &Extent) -> Result<(), Error> {
        let fail = |reason: &str| Error::Format(format!("subarea: {reason}"));
        if self.areas.len() > MAX_AREAS
            || self.masks.len() > MAX_MASKS
            || self.runs.len() > MAX_RUNS
        {
            return Err(Error::Limit("subarea table".into()));
        }
        if self
            .masks
            .iter()
            .map(|m| u64::from(m.source_tiles))
            .sum::<u64>()
            > MAX_TILES as u64
        {
            return Err(Error::Limit("subarea source tiles".into()));
        }
        if self == &Self::default() {
            return Ok(());
        }
        let hash = |s: &str| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if self.client_version != "15.30"
            || !hash(&self.candidate_sha256)
            || self.areas.is_empty()
            || self.masks.is_empty()
            || !self.areas.windows(2).all(|p| p[0].id < p[1].id)
            || !self.masks.windows(2).all(|p| p[0].subarea < p[1].subarea)
        {
            return Err(fail("identity or order"));
        }
        let area = |id| {
            self.areas
                .binary_search_by_key(&id, |a| a.id)
                .ok()
                .map(|i| &self.areas[i])
        };
        let mut keys = std::collections::BTreeSet::new();
        for a in &self.areas {
            if a.id == 0
                || !a.key.starts_with("oteryn:content.area.")
                || a.key.len() > 256
                || !keys.insert(&a.key)
                || a.name.is_empty()
                || a.name.len() > 256
                || !hash(&a.source_record_sha256)
                || a.parent
                    .is_some_and(|p| p == a.id || area(p).is_none_or(|p| p.parent.is_some()))
            {
                return Err(fail("area identity or parent"));
            }
        }
        let mut counts = std::collections::BTreeMap::<u32, usize>::new();
        let mut total = 0;
        let mut previous: Option<&Run> = None;
        for r in &self.runs {
            if r.begin >= r.end
                || !world.contains(r.begin, r.y, r.floor)
                || !world.contains(r.end - 1, r.y, r.floor)
                || previous.is_some_and(|p| {
                    (p.floor, p.y, p.begin) >= (r.floor, r.y, r.begin)
                        || (p.floor == r.floor && p.y == r.y && p.end > r.begin)
                })
                || self
                    .masks
                    .binary_search_by_key(&r.subarea, |m| m.subarea)
                    .is_err()
            {
                return Err(fail("run bounds, overlap or membership"));
            }
            let n = usize::from(r.end - r.begin);
            total += n;
            if total > MAX_TILES {
                return Err(Error::Limit("subarea tiles".into()));
            }
            *counts.entry(r.subarea).or_default() += n;
            previous = Some(r);
        }
        for m in &self.masks {
            if area(m.subarea).is_none_or(|a| a.parent.is_none())
                || !hash(&m.compressed_sha256)
                || !hash(&m.bmp_sha256)
                || !hash(&m.source_record_sha256)
                || m.asset
                    != format!(
                        "content/assets/files/subarea-{:04}-{}.bmp.lzma",
                        m.subarea, m.bmp_sha256
                    )
                || m.source_tiles == 0
                || m.source_tiles as usize > MAX_TILES
                || m.admitted_tiles > m.source_tiles
                || counts.get(&m.subarea).copied().unwrap_or(0) != m.admitted_tiles as usize
            {
                return Err(fail("mask provenance or counts"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn table() -> Table {
        let hash = "a".repeat(64);
        Table {
            client_version: "15.30".into(),
            candidate_sha256: hash.clone(),
            areas: vec![
                Area {
                    id: 23,
                    key: "oteryn:content.area.region.hrodmir".into(),
                    name: "Hrodmir".into(),
                    parent: None,
                    source_record_sha256: hash.clone(),
                },
                Area {
                    id: 109,
                    key: "oteryn:content.area.subregion.nibelor".into(),
                    name: "Nibelor".into(),
                    parent: Some(23),
                    source_record_sha256: hash.clone(),
                },
            ],
            masks: vec![Mask {
                subarea: 109,
                asset: format!("content/assets/files/subarea-0109-{hash}.bmp.lzma"),
                compressed_sha256: hash.clone(),
                bmp_sha256: hash.clone(),
                source_record_sha256: hash,
                source_tiles: 5,
                admitted_tiles: 3,
            }],
            runs: vec![Run {
                floor: -7,
                y: 10,
                begin: 20,
                end: 23,
                subarea: 109,
            }],
        }
    }
    fn world() -> Extent {
        Extent {
            min_x: 0,
            min_y: 0,
            max_x: 32,
            max_y: 32,
            floors: vec![-7],
        }
    }

    #[test]
    fn subarea_reader_rejects_overlap_bounds_bad_parents_provenance_and_counts() {
        let valid = table();
        assert_eq!(valid.validate(&world()), Ok(()));
        assert_eq!(valid.resolve(20, 10, -7), Some((23, 109)));
        assert_eq!(valid.resolve(22, 10, -7), Some((23, 109)));
        assert_eq!(valid.resolve(23, 10, -7), None);
        assert_eq!(valid.resolve(20, 10, -6), None);
        let mut invalid = Vec::new();
        let mut t = valid.clone();
        t.runs.push(t.runs[0]);
        invalid.push(t);
        let mut t = valid.clone();
        t.runs[0].end = 33;
        invalid.push(t);
        let mut t = valid.clone();
        t.areas[1].parent = Some(109);
        invalid.push(t);
        let mut t = valid.clone();
        t.areas[1].parent = Some(999);
        invalid.push(t);
        let mut t = valid.clone();
        t.masks[0].admitted_tiles = 4;
        invalid.push(t);
        let mut t = valid.clone();
        t.masks[0].asset = "../mask".into();
        invalid.push(t);
        let mut t = valid.clone();
        t.masks[0].bmp_sha256 = "b".repeat(64);
        invalid.push(t);
        let mut t = valid.clone();
        t.runs[0].subarea = 999;
        invalid.push(t);
        let mut t = valid.clone();
        t.areas.reverse();
        invalid.push(t);
        let mut t = valid.clone();
        t.client_version = "unknown".into();
        invalid.push(t);
        for t in invalid {
            assert!(t.validate(&world()).is_err(), "{t:?}");
        }
    }

    #[test]
    fn subarea_reader_checks_limits_and_fails_closed_on_partial_tables() {
        let mut t = table();
        t.runs = vec![t.runs[0]; MAX_RUNS + 1];
        assert!(matches!(t.validate(&world()), Err(Error::Limit(_))));
        let mut t = table();
        t.masks[0].source_tiles = MAX_TILES as u32 + 1;
        assert!(matches!(t.validate(&world()), Err(Error::Limit(_))));
        assert_eq!(Table::default().validate(&world()), Ok(()));
        let t = Table {
            candidate_sha256: "a".repeat(64),
            ..Table::default()
        };
        assert!(t.validate(&world()).is_err());
    }
}
