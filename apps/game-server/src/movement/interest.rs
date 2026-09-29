//! Deterministic interest area, canonical ordering, bounded visibility query and interest diff.
//!
//! Pure server logic for the `MOVE-RL-11-VISIBILITY-V1` decision (#162, decision document
//! `docs/architecture/reviews/OTERYN_GAME_MOVE_RL11_VISIBILITY_DECISION_2026-09-28.md` §4.1-§4.4).
//! It has no production caller, no wire representation and no entity payload: an entity here is
//! only an identity, a position and an opaque change stamp. The interest index enumerates
//! candidates in the canonical order (floor distance, then Chebyshev distance on the observer's
//! plane, then identity bytes), so a cutoff always keeps the nearest and the result never
//! depends on insertion order.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::error::Error;
use std::fmt;
use std::ops::Bound;

/// `MOVE-RL-08`: enter, leave and update entries in one delta; a larger change is a resync.
pub(crate) const INTEREST_DELTA_ENTRIES_MAX: usize = 256;
/// `MOVE-RL-09`: candidates examined by one visibility query.
pub(crate) const VISIBILITY_QUERY_CANDIDATES_MAX: usize = 1024;
/// `MOVE-RL-10` (equal to `MOVE-RL-11`): results returned by one visibility query.
pub(crate) const VISIBILITY_QUERY_RESULTS_MAX: usize = 256;

pub(crate) const VIEW_WIDTH_MIN: u16 = 15;
pub(crate) const VIEW_WIDTH_MAX: u16 = 36;
pub(crate) const VIEW_HEIGHT_MIN: u16 = 11;
pub(crate) const VIEW_HEIGHT_MAX: u16 = 28;

/// Tibia floors: 0 is the top, 7 is the ground floor, 15 is the deepest.
const FLOOR_GROUND: i16 = 7;
const FLOOR_MAX: i16 = 15;
/// Floors visible above and below an underground observer (`MOVE-VIEW-FLOORS`: 5 underground).
const UNDERGROUND_FLOOR_REACH: i16 = 2;

/// Opaque entity identity; compared bytewise (canonical order tie-break).
pub(crate) type EntityIdentity = [u8; 16];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InterestError {
    FloorOutOfRange,
    WidthOutOfRange,
    HeightOutOfRange,
    ObserverNotIndexed,
}

impl fmt::Display for InterestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::FloorOutOfRange => "floor is outside 0..=15",
            Self::WidthOutOfRange => "view width is outside 15..=36",
            Self::HeightOutOfRange => "view height is outside 11..=28",
            Self::ObserverNotIndexed => "observer is not in the interest index",
        })
    }
}

impl Error for InterestError {}

/// A tile position on one floor. The floor is validated to `0..=15`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct VisibilityPosition {
    x: i32,
    y: i32,
    floor: i16,
}

impl VisibilityPosition {
    pub(crate) fn new(x: i32, y: i32, floor: i16) -> Result<Self, InterestError> {
        if (0..=FLOOR_MAX).contains(&floor) {
            Ok(Self { x, y, floor })
        } else {
            Err(InterestError::FloorOutOfRange)
        }
    }
}

/// Immutable per-Channel view size. `REFERENCE` is 18 x 14.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VisibilitySettings {
    width: u16,
    height: u16,
}

impl VisibilitySettings {
    pub(crate) const REFERENCE: Self = Self {
        width: 18,
        height: 14,
    };

    pub(crate) fn new(width: u16, height: u16) -> Result<Self, InterestError> {
        if !(VIEW_WIDTH_MIN..=VIEW_WIDTH_MAX).contains(&width) {
            return Err(InterestError::WidthOutOfRange);
        }
        if !(VIEW_HEIGHT_MIN..=VIEW_HEIGHT_MAX).contains(&height) {
            return Err(InterestError::HeightOutOfRange);
        }
        Ok(Self { width, height })
    }

    pub(crate) const fn width(self) -> u16 {
        self.width
    }

    pub(crate) const fn height(self) -> u16 {
        self.height
    }

    const fn west(self) -> i64 {
        ((self.width as i64) - 1) / 2
    }

    const fn east(self) -> i64 {
        (self.width as i64) - 1 - self.west()
    }

    const fn north(self) -> i64 {
        ((self.height as i64) - 1) / 2
    }

    const fn south(self) -> i64 {
        (self.height as i64) - 1 - self.north()
    }

    /// Largest Chebyshev distance on the observer plane inside the area.
    const fn ring_max(self) -> i64 {
        let horizontal = if self.west() > self.east() {
            self.west()
        } else {
            self.east()
        };
        let vertical = if self.north() > self.south() {
            self.north()
        } else {
            self.south()
        };
        if horizontal > vertical {
            horizontal
        } else {
            vertical
        }
    }

    /// Whether an observer at `observer` sees the tile `target`.
    pub(crate) fn can_see(self, observer: VisibilityPosition, target: VisibilityPosition) -> bool {
        self.plane_offsets(observer, target).is_some()
    }

    /// Offsets of `target` on the observer plane (floor offset removed), when visible.
    fn plane_offsets(
        self,
        observer: VisibilityPosition,
        target: VisibilityPosition,
    ) -> Option<(i64, i64)> {
        if !floor_visible(observer.floor, target.floor) {
            return None;
        }
        let d = i64::from(observer.floor) - i64::from(target.floor);
        let dx = i64::from(target.x) - d - i64::from(observer.x);
        let dy = i64::from(target.y) - d - i64::from(observer.y);
        ((-self.west()..=self.east()).contains(&dx) && (-self.north()..=self.south()).contains(&dy))
            .then_some((dx, dy))
    }

    /// The canonical order key relative to `observer`; visible targets only.
    #[cfg(test)]
    fn canonical_key(
        self,
        observer: VisibilityPosition,
        entity: &InterestEntity,
    ) -> Option<(i64, i64, EntityIdentity)> {
        let (dx, dy) = self.plane_offsets(observer, entity.position)?;
        let floor_distance = (i64::from(observer.floor) - i64::from(entity.position.floor)).abs();
        Some((floor_distance, dx.abs().max(dy.abs()), entity.identity))
    }
}

/// Whether floor `floor` is visible from an observer on `observer_floor`.
fn floor_visible(observer_floor: i16, floor: i16) -> bool {
    if !(0..=FLOOR_MAX).contains(&floor) {
        return false;
    }
    if observer_floor <= FLOOR_GROUND {
        floor <= FLOOR_GROUND
    } else {
        (observer_floor - floor).abs() <= UNDERGROUND_FLOOR_REACH
    }
}

/// Identity, position and an opaque change stamp. Payload fields belong to a later slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InterestEntity {
    pub(crate) identity: EntityIdentity,
    pub(crate) position: VisibilityPosition,
    pub(crate) revision: u64,
}

type CellKey = (i16, i32, i32);

fn cell_key(position: VisibilityPosition) -> CellKey {
    (position.floor, position.x, position.y)
}

/// Spatial index over entity positions. Iteration order is independent of insertion order.
#[derive(Debug, Default)]
pub(crate) struct InterestIndex {
    entities: BTreeMap<EntityIdentity, InterestEntity>,
    cells: BTreeMap<CellKey, BTreeSet<EntityIdentity>>,
}

/// The bounded result of one visibility query: the own actor first, then canonical order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VisibilityQuery {
    entities: Vec<InterestEntity>,
    examined: usize,
}

impl VisibilityQuery {
    pub(crate) fn entities(&self) -> &[InterestEntity] {
        &self.entities
    }

    /// Candidates examined, including the own actor.
    pub(crate) const fn examined(&self) -> usize {
        self.examined
    }
}

impl InterestIndex {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces the entity with the same identity.
    pub(crate) fn upsert(&mut self, entity: InterestEntity) {
        self.remove(&entity.identity);
        self.cells
            .entry(cell_key(entity.position))
            .or_default()
            .insert(entity.identity);
        self.entities.insert(entity.identity, entity);
    }

    pub(crate) fn remove(&mut self, identity: &EntityIdentity) -> Option<InterestEntity> {
        let entity = self.entities.remove(identity)?;
        let key = cell_key(entity.position);
        if let Some(set) = self.cells.get_mut(&key) {
            set.remove(identity);
            if set.is_empty() {
                self.cells.remove(&key);
            }
        }
        Some(entity)
    }

    pub(crate) fn get(&self, identity: &EntityIdentity) -> Option<&InterestEntity> {
        self.entities.get(identity)
    }

    pub(crate) fn query(
        &self,
        observer: &EntityIdentity,
        settings: VisibilitySettings,
    ) -> Result<VisibilityQuery, InterestError> {
        self.query_with(observer, settings, |_| true)
    }

    /// Enumerates candidates in canonical order, ring by ring outwards from the observer, and
    /// stops at `VISIBILITY_QUERY_RESULTS_MAX` results or `VISIBILITY_QUERY_CANDIDATES_MAX`
    /// examined candidates. `accept` may reject a candidate (it still counts as examined); the
    /// own actor is always included and counts as one result and one examined candidate.
    pub(crate) fn query_with(
        &self,
        observer: &EntityIdentity,
        settings: VisibilitySettings,
        mut accept: impl FnMut(&InterestEntity) -> bool,
    ) -> Result<VisibilityQuery, InterestError> {
        let own = *self
            .entities
            .get(observer)
            .ok_or(InterestError::ObserverNotIndexed)?;
        let me = own.position;
        let mut entities =
            Vec::with_capacity(self.entities.len().min(VISIBILITY_QUERY_RESULTS_MAX));
        entities.push(own);
        let mut examined = 1_usize;

        'query: for floor_distance in 0..=FLOOR_MAX {
            let floors = [me.floor - floor_distance, me.floor + floor_distance];
            let floors = if floor_distance == 0 {
                &floors[..1]
            } else {
                &floors[..]
            };
            if !floors.iter().any(|floor| floor_visible(me.floor, *floor)) {
                continue;
            }
            for ring in 0..=settings.ring_max() {
                if entities.len() >= VISIBILITY_QUERY_RESULTS_MAX
                    || examined >= VISIBILITY_QUERY_CANDIDATES_MAX
                {
                    break 'query;
                }
                let budget = VISIBILITY_QUERY_CANDIDATES_MAX - examined;
                let mut cells: Vec<&BTreeSet<EntityIdentity>> = Vec::new();
                for floor in floors
                    .iter()
                    .copied()
                    .filter(|f| floor_visible(me.floor, *f))
                {
                    let d = i64::from(me.floor) - i64::from(floor);
                    for_each_ring_cell(settings, ring, |dx, dy| {
                        let (Ok(x), Ok(y)) = (
                            i32::try_from(i64::from(me.x) + dx + d),
                            i32::try_from(i64::from(me.y) + dy + d),
                        ) else {
                            return;
                        };
                        if let Some(ids) = self.cells.get(&(floor, x, y)) {
                            cells.push(ids);
                        }
                    });
                }
                for id in MergeBounded::new(&cells, observer, budget) {
                    examined += 1;
                    if let Some(entity) = self.entities.get(&id)
                        && accept(entity)
                    {
                        entities.push(*entity);
                    }
                    if entities.len() >= VISIBILITY_QUERY_RESULTS_MAX
                        || examined >= VISIBILITY_QUERY_CANDIDATES_MAX
                    {
                        break 'query;
                    }
                }
            }
        }
        Ok(VisibilityQuery { entities, examined })
    }
}

/// Lazily yields, in ascending order, the `cap` smallest identities across the cells of one
/// canonical group, excluding `observer`, by a k-way merge. Only a yielded identity is visited,
/// so the group costs exactly as many visits as the caller pulls, at most `cap`, however dense
/// its cells are and however early the caller stops; finding a cell's next identity is an
/// ordered-set range lookup, not a walk. Cells hold disjoint identities.
struct MergeBounded<'a> {
    cells: &'a [&'a BTreeSet<EntityIdentity>],
    observer: &'a EntityIdentity,
    heads: BinaryHeap<Reverse<(EntityIdentity, usize)>>,
    remaining: usize,
}

impl<'a> MergeBounded<'a> {
    fn new(
        cells: &'a [&'a BTreeSet<EntityIdentity>],
        observer: &'a EntityIdentity,
        cap: usize,
    ) -> Self {
        let mut merge = Self {
            cells,
            observer,
            heads: BinaryHeap::with_capacity(cells.len()),
            remaining: cap,
        };
        for cell in 0..cells.len() {
            merge.push_next(cell, Bound::Unbounded);
        }
        merge
    }

    fn push_next(&mut self, cell: usize, after: Bound<&EntityIdentity>) {
        let observer = self.observer;
        if let Some(id) = self.cells[cell]
            .range::<EntityIdentity, _>((after, Bound::Unbounded))
            .find(|id| *id != observer)
        {
            self.heads.push(Reverse((*id, cell)));
        }
    }
}

impl Iterator for MergeBounded<'_> {
    type Item = EntityIdentity;

    fn next(&mut self) -> Option<EntityIdentity> {
        if self.remaining == 0 {
            return None;
        }
        let Reverse((id, cell)) = self.heads.pop()?;
        self.remaining -= 1;
        self.push_next(cell, Bound::Excluded(&id));
        Some(id)
    }
}

/// Calls `visit(dx, dy)` for each in-area cell at Chebyshev distance `ring` on the observer plane.
fn for_each_ring_cell(settings: VisibilitySettings, ring: i64, mut visit: impl FnMut(i64, i64)) {
    let (west, east) = (settings.west(), settings.east());
    let (north, south) = (settings.north(), settings.south());
    let dy_min = (-ring).max(-north);
    let dy_max = ring.min(south);
    for dy in dy_min..=dy_max {
        if dy.abs() == ring {
            for dx in (-ring).max(-west)..=ring.min(east) {
                visit(dx, dy);
            }
        } else {
            for dx in [-ring, ring] {
                if (-west..=east).contains(&dx) {
                    visit(dx, dy);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InterestChange {
    Enter(InterestEntity),
    Leave(EntityIdentity),
    Update(InterestEntity),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InterestDiff {
    /// At most `INTEREST_DELTA_ENTRIES_MAX` entries, ascending by identity.
    Delta(Vec<InterestChange>),
    /// The change exceeded the delta ceiling: the full current snapshot.
    Resync(Vec<InterestEntity>),
}

/// Diff between the previous and current result of one observer. An entity present in both
/// results is an update when its position or revision changed.
pub(crate) fn diff_interest(previous: &VisibilityQuery, current: &VisibilityQuery) -> InterestDiff {
    let previous: BTreeMap<_, _> = previous.entities.iter().map(|e| (e.identity, *e)).collect();
    let current_map: BTreeMap<_, _> = current.entities.iter().map(|e| (e.identity, *e)).collect();
    let mut changes = Vec::new();
    let identities: BTreeSet<_> = previous.keys().chain(current_map.keys()).copied().collect();
    for identity in identities {
        let change = match (previous.get(&identity), current_map.get(&identity)) {
            (None, Some(now)) => InterestChange::Enter(*now),
            (Some(_), None) => InterestChange::Leave(identity),
            (Some(before), Some(now)) if before != now => InterestChange::Update(*now),
            _ => continue,
        };
        if changes.len() == INTEREST_DELTA_ENTRIES_MAX {
            return InterestDiff::Resync(current.entities.clone());
        }
        changes.push(change);
    }
    InterestDiff::Delta(changes)
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn Error>>;

    fn id(n: u32) -> EntityIdentity {
        let mut bytes = [0_u8; 16];
        bytes[12..].copy_from_slice(&n.to_be_bytes());
        bytes
    }

    fn pos(x: i32, y: i32, floor: i16) -> Result<VisibilityPosition, InterestError> {
        VisibilityPosition::new(x, y, floor)
    }

    fn entity(n: u32, x: i32, y: i32, floor: i16) -> Result<InterestEntity, InterestError> {
        Ok(InterestEntity {
            identity: id(n),
            position: pos(x, y, floor)?,
            revision: 0,
        })
    }

    /// (x_min, x_max, y_min, y_max) relative to the observer over which `target_floor` is visible.
    fn extent(
        settings: VisibilitySettings,
        observer_floor: i16,
        target_floor: i16,
    ) -> Result<Option<(i32, i32, i32, i32)>, InterestError> {
        let observer = pos(1000, 1000, observer_floor)?;
        let mut found: Option<(i32, i32, i32, i32)> = None;
        for x in 900..1100 {
            for y in 900..1100 {
                if settings.can_see(observer, pos(x, y, target_floor)?) {
                    let (rx, ry) = (x - 1000, y - 1000);
                    found = Some(match found {
                        None => (rx, rx, ry, ry),
                        Some((a, b, c, d)) => (a.min(rx), b.max(rx), c.min(ry), d.max(ry)),
                    });
                }
            }
        }
        Ok(found)
    }

    #[test]
    fn constants_are_the_registered_row_values() {
        assert_eq!(INTEREST_DELTA_ENTRIES_MAX, 256);
        assert_eq!(VISIBILITY_QUERY_CANDIDATES_MAX, 1024);
        assert_eq!(VISIBILITY_QUERY_RESULTS_MAX, 256);
    }

    #[test]
    fn settings_accept_bounds_and_reject_outside() -> TestResult {
        for (w, h) in [(15, 11), (36, 28), (15, 28), (36, 11), (18, 14)] {
            let s = VisibilitySettings::new(w, h)?;
            assert_eq!((s.width(), s.height()), (w, h));
        }
        assert_eq!(
            VisibilitySettings::REFERENCE,
            VisibilitySettings::new(18, 14)?
        );
        assert_eq!(
            VisibilitySettings::new(14, 11),
            Err(InterestError::WidthOutOfRange)
        );
        assert_eq!(
            VisibilitySettings::new(37, 14),
            Err(InterestError::WidthOutOfRange)
        );
        assert_eq!(
            VisibilitySettings::new(15, 10),
            Err(InterestError::HeightOutOfRange)
        );
        assert_eq!(
            VisibilitySettings::new(15, 29),
            Err(InterestError::HeightOutOfRange)
        );
        assert_eq!(
            VisibilitySettings::new(0, 0),
            Err(InterestError::WidthOutOfRange)
        );
        Ok(())
    }

    #[test]
    fn position_rejects_floors_outside_zero_to_fifteen() -> TestResult {
        assert!(VisibilityPosition::new(0, 0, 0).is_ok());
        assert!(VisibilityPosition::new(0, 0, 15).is_ok());
        assert_eq!(
            VisibilityPosition::new(0, 0, -1),
            Err(InterestError::FloorOutOfRange)
        );
        assert_eq!(
            VisibilityPosition::new(0, 0, 16),
            Err(InterestError::FloorOutOfRange)
        );
        Ok(())
    }

    #[test]
    fn area_offsets_at_reference_min_and_max_sizes() -> TestResult {
        // west, east, north, south per the decision's §4.1 formula.
        let cases = [
            (VisibilitySettings::REFERENCE, (-8, 9, -6, 7)),
            (VisibilitySettings::new(15, 11)?, (-7, 7, -5, 5)),
            (VisibilitySettings::new(36, 28)?, (-17, 18, -13, 14)),
        ];
        for (settings, expected) in cases {
            assert_eq!(extent(settings, 7, 7)?, Some(expected));
            // Area size equals the configured width x height.
            let width = i32::from(settings.width());
            let height = i32::from(settings.height());
            assert_eq!(
                (expected.1 - expected.0 + 1, expected.3 - expected.2 + 1),
                (width, height)
            );
        }
        Ok(())
    }

    #[test]
    fn above_ground_observer_sees_floors_zero_to_seven_only() -> TestResult {
        let s = VisibilitySettings::REFERENCE;
        for observer_floor in [7, 3, 0] {
            for floor in 0..=15 {
                let visible = extent(s, observer_floor, floor)?.is_some();
                assert_eq!(
                    visible,
                    floor <= 7,
                    "observer {observer_floor} floor {floor}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn underground_observer_sees_two_floors_each_way_clamped_at_fifteen() -> TestResult {
        let s = VisibilitySettings::REFERENCE;
        let visible = |observer: i16| -> Result<Vec<i16>, InterestError> {
            let mut floors = Vec::new();
            for floor in 0..=15 {
                if extent(s, observer, floor)?.is_some() {
                    floors.push(floor);
                }
            }
            Ok(floors)
        };
        assert_eq!(visible(10)?, vec![8, 9, 10, 11, 12]);
        assert_eq!(visible(8)?, vec![6, 7, 8, 9, 10]);
        assert_eq!(visible(14)?, vec![12, 13, 14, 15]);
        assert_eq!(visible(15)?, vec![13, 14, 15]);
        Ok(())
    }

    #[test]
    fn floor_offset_shifts_both_axes_with_the_observer_minus_target_sign() -> TestResult {
        let s = VisibilitySettings::REFERENCE;
        // Observer z=7, tile z=8 (not visible above ground); use underground observers for signs.
        assert_eq!(extent(s, 7, 8)?, None);
        // Observer z=10: tile z=11 has d=-1 -> x in [-9, 8], y in [-7, 6].
        assert_eq!(extent(s, 10, 11)?, Some((-9, 8, -7, 6)));
        // Tile z=12 has d=-2 -> x in [-10, 7], y in [-8, 5].
        assert_eq!(extent(s, 10, 12)?, Some((-10, 7, -8, 5)));
        // Tile z=9 has d=+1 -> x in [-7, 10], y in [-5, 8].
        assert_eq!(extent(s, 10, 9)?, Some((-7, 10, -5, 8)));
        // Above ground observer z=5, tile z=7: d=-2.
        assert_eq!(extent(s, 5, 7)?, Some((-10, 7, -8, 5)));
        // Above ground observer z=7, tile z=5: d=+2.
        assert_eq!(extent(s, 7, 5)?, Some((-6, 11, -4, 9)));
        // Observer z=7 tile z=6: d=+1.
        assert_eq!(extent(s, 7, 6)?, Some((-7, 10, -5, 8)));
        Ok(())
    }

    /// Observer id(u32::MAX) at (100, 100, 7).
    fn index_with_observer() -> Result<InterestIndex, InterestError> {
        let mut index = InterestIndex::new();
        index.upsert(entity(u32::MAX, 100, 100, 7)?);
        Ok(index)
    }

    #[test]
    fn query_requires_an_indexed_observer() {
        let index = InterestIndex::new();
        assert_eq!(
            index.query(&id(1), VisibilitySettings::REFERENCE),
            Err(InterestError::ObserverNotIndexed)
        );
    }

    #[test]
    fn query_returns_max_results_and_excludes_the_farthest_beyond_it() -> TestResult {
        let s = VisibilitySettings::REFERENCE;
        let mut index = index_with_observer()?;
        // 254 near others (ring <= 4) plus one far entity on ring 9 = 255 others + own = 256.
        for n in 0..254_u32 {
            let (dx, dy) = ((n % 5) as i32 - 2, ((n / 5) % 5) as i32 - 2);
            index.upsert(entity(n, 100 + dx, 100 + dy, 7)?);
        }
        index.upsert(entity(1000, 109, 100, 7)?);
        let at_max = index.query(&id(u32::MAX), s)?;
        assert_eq!(at_max.entities().len(), VISIBILITY_QUERY_RESULTS_MAX);
        assert!(at_max.entities().iter().any(|e| e.identity == id(1000)));

        // Max + 1: a 257th visible entity nearer than entity 1000 (ring 7 against ring 9) is
        // added; the farthest visible entity, 1000, loses the last slot.
        index.upsert(entity(1001, 100, 107, 7)?);
        let over = index.query(&id(u32::MAX), s)?;
        assert_eq!(over.entities().len(), VISIBILITY_QUERY_RESULTS_MAX);
        let ids: Vec<_> = over.entities().iter().map(|e| e.identity).collect();
        assert!(ids.contains(&id(1001)));
        assert!(!ids.contains(&id(1000)));
        assert_eq!(over.examined(), VISIBILITY_QUERY_RESULTS_MAX);
        Ok(())
    }

    #[test]
    fn own_actor_is_included_and_counted_with_more_nearer_candidates_than_the_ceiling() -> TestResult
    {
        let s = VisibilitySettings::REFERENCE;
        let mut index = index_with_observer()?;
        // 300 others on the observer's own tile, all with identities below the observer's.
        for n in 0..300_u32 {
            index.upsert(entity(n, 100, 100, 7)?);
        }
        let result = index.query(&id(u32::MAX), s)?;
        assert_eq!(result.entities().len(), VISIBILITY_QUERY_RESULTS_MAX);
        assert_eq!(result.entities()[0].identity, id(u32::MAX));
        let others: Vec<_> = result.entities()[1..].iter().map(|e| e.identity).collect();
        let expected: Vec<_> = (0..255_u32).map(id).collect();
        assert_eq!(others, expected);
        Ok(())
    }

    #[test]
    fn floor_distance_orders_before_planar_distance() -> TestResult {
        let s = VisibilitySettings::REFERENCE;
        let mut index = index_with_observer()?;
        index.upsert(entity(1, 100 + 8, 100, 7)?); // same floor, ring 8
        index.upsert(entity(2, 100 - 1, 100 - 1, 6)?); // floor distance 1, plane offset 0
        index.upsert(entity(3, 100, 100, 7)?); // same tile
        let result = index.query(&id(u32::MAX), s)?;
        let ids: Vec<_> = result.entities().iter().map(|e| e.identity).collect();
        // Entity 2 is one floor away, so it sorts after every same-floor candidate.
        assert_eq!(ids, vec![id(u32::MAX), id(3), id(1), id(2)]);
        Ok(())
    }

    #[test]
    fn invisible_positions_and_floors_are_excluded() -> TestResult {
        let s = VisibilitySettings::REFERENCE;
        let mut index = index_with_observer()?;
        index.upsert(entity(1, 100 + 9, 100 + 7, 7)?); // corner: visible
        index.upsert(entity(2, 100 + 10, 100, 7)?); // one east too far
        index.upsert(entity(3, 100, 100 - 7, 7)?); // one north too far
        index.upsert(entity(4, 100, 100, 8)?); // below ground floor for a floor 7 observer
        let ids: Vec<_> = index
            .query(&id(u32::MAX), s)?
            .entities()
            .iter()
            .map(|e| e.identity)
            .collect();
        assert_eq!(ids, vec![id(u32::MAX), id(1)]);
        Ok(())
    }

    /// Deterministic pseudo-random scatter (no RNG dependency).
    fn scatter(count: u32) -> Result<Vec<InterestEntity>, InterestError> {
        let mut state = 0x2545_F491_4F6C_DD1D_u64;
        let mut next = move |modulus: u64| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) % modulus
        };
        (0..count)
            .map(|n| {
                let x = 100 - 8 + next(18) as i32;
                let y = 100 - 6 + next(14) as i32;
                let floor = next(8) as i16;
                entity(n, x, y, floor)
            })
            .collect()
    }

    #[test]
    fn over_candidate_ceiling_selects_canonical_nearest_regardless_of_insertion_order() -> TestResult
    {
        let s = VisibilitySettings::REFERENCE;
        let observer = entity(u32::MAX, 100, 100, 7)?;
        let others = scatter(1500)?;

        let build = |order: &[usize]| {
            let mut index = InterestIndex::new();
            for i in order {
                index.upsert(others[*i]);
            }
            index.upsert(observer);
            index
        };
        let ascending: Vec<usize> = (0..others.len()).collect();
        // 1500 is coprime with 7: a fixed permutation.
        let permuted: Vec<usize> = (0..others.len())
            .map(|i| (i * 7 + 3) % others.len())
            .collect();
        let reversed: Vec<usize> = ascending.iter().rev().copied().collect();

        let a = build(&ascending).query(&id(u32::MAX), s)?;
        let b = build(&permuted).query(&id(u32::MAX), s)?;
        let c = build(&reversed).query(&id(u32::MAX), s)?;
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a.entities().len(), VISIBILITY_QUERY_RESULTS_MAX);

        // Cross-check against a brute-force canonical sort.
        let mut expected: Vec<_> = others
            .iter()
            .filter_map(|e| s.canonical_key(observer.position, e).map(|key| (key, *e)))
            .collect();
        expected.sort_by_key(|(key, _)| *key);
        let expected: Vec<_> = std::iter::once(observer)
            .chain(
                expected
                    .into_iter()
                    .take(VISIBILITY_QUERY_RESULTS_MAX - 1)
                    .map(|(_, e)| e),
            )
            .collect();
        assert_eq!(a.entities(), expected.as_slice());
        Ok(())
    }

    #[test]
    fn candidate_ceiling_stops_examination_at_max_and_not_at_max_plus_one() -> TestResult {
        let s = VisibilitySettings::REFERENCE;
        for total_candidates in [1023_u32, 1024, 1025, 1500] {
            // Observer plus `total_candidates - 1` others; the filter rejects every other.
            let mut index = index_with_observer()?;
            for n in 0..(total_candidates - 1) {
                index.upsert(entity(n, 100 + (n % 5) as i32, 100, 7)?);
            }
            let mut calls = 0_usize;
            let result = index.query_with(&id(u32::MAX), s, |_| {
                calls += 1;
                false
            })?;
            let expected = (total_candidates as usize).min(VISIBILITY_QUERY_CANDIDATES_MAX);
            assert_eq!(result.examined(), expected, "candidates {total_candidates}");
            assert_eq!(calls, expected - 1);
            assert_eq!(result.entities().len(), 1);
        }
        Ok(())
    }

    #[test]
    fn candidate_ceiling_selection_is_canonical_when_the_filter_rejects() -> TestResult {
        let s = VisibilitySettings::REFERENCE;
        let mut index = index_with_observer()?;
        for n in 0..1200_u32 {
            index.upsert(entity(n, 100, 100, 7)?);
        }
        let mut seen = Vec::new();
        index.query_with(&id(u32::MAX), s, |e| {
            seen.push(e.identity);
            false
        })?;
        let expected: Vec<_> = (0..1023_u32).map(id).collect();
        assert_eq!(seen, expected);
        Ok(())
    }

    #[test]
    fn dense_cell_scan_visits_at_most_the_budget_and_keeps_the_canonical_first() -> TestResult {
        let dense = 5000_u32;
        let mut index = index_with_observer()?;
        for n in (0..dense).rev() {
            index.upsert(entity(n, 100, 100, 7)?);
        }
        let mut seen = Vec::new();
        let result = index.query_with(&id(u32::MAX), VisibilitySettings::REFERENCE, |e| {
            seen.push(e.identity);
            false
        })?;
        assert!(result.examined() <= VISIBILITY_QUERY_CANDIDATES_MAX);
        let expected: Vec<_> = (0..(VISIBILITY_QUERY_CANDIDATES_MAX as u32 - 1))
            .map(id)
            .collect();
        assert_eq!(seen, expected);

        // Several dense cells of one canonical group share a single budget: exactly `cap`
        // identities are visited in total and they are the canonical first `cap`.
        let cells: Vec<BTreeSet<_>> = (0..4_u32)
            .map(|c| (0..dense).map(|n| id(n * 4 + c)).collect())
            .collect();
        let refs: Vec<_> = cells.iter().collect();
        let observer = id(u32::MAX);
        let group: Vec<_> =
            MergeBounded::new(&refs, &observer, VISIBILITY_QUERY_CANDIDATES_MAX).collect();
        assert_eq!(group.len(), VISIBILITY_QUERY_CANDIDATES_MAX);
        let expected: Vec<_> = (0..VISIBILITY_QUERY_CANDIDATES_MAX as u32)
            .map(id)
            .collect();
        assert_eq!(group, expected);

        // The observer is skipped without costing budget, and a small group is fully returned.
        let small: BTreeSet<_> = [id(2), id(5), id(u32::MAX)].into_iter().collect();
        let other: BTreeSet<_> = [id(1), id(3)].into_iter().collect();
        let group: Vec<_> = MergeBounded::new(&[&small, &other], &observer, 10).collect();
        assert_eq!(group, vec![id(1), id(2), id(3), id(5)]);
        Ok(())
    }

    #[test]
    fn result_ceiling_stops_the_merge_at_the_pull_that_reaches_it() -> TestResult {
        // Dense group spread over the four ring-1 cells around the observer; every candidate is
        // accepted, so the 256-result ceiling is hit long before the 1,024-candidate ceiling.
        let mut index = index_with_observer()?;
        for n in 0..4000_u32 {
            let (x, y) = [(101, 100), (99, 100), (100, 101), (100, 99)][(n % 4) as usize];
            index.upsert(entity(n, x, y, 7)?);
        }
        let mut accepted = 0_usize;
        let result = index.query_with(&id(u32::MAX), VisibilitySettings::REFERENCE, |_| {
            accepted += 1;
            true
        })?;
        assert_eq!(result.examined(), VISIBILITY_QUERY_RESULTS_MAX);
        assert_eq!(accepted, VISIBILITY_QUERY_RESULTS_MAX - 1);
        let ids: Vec<_> = result.entities().iter().map(|e| e.identity).collect();
        let expected: Vec<_> = std::iter::once(id(u32::MAX))
            .chain((0..VISIBILITY_QUERY_RESULTS_MAX as u32 - 1).map(id))
            .collect();
        assert_eq!(ids, expected);
        Ok(())
    }

    #[test]
    fn upsert_moves_an_entity_and_remove_deletes_it() -> TestResult {
        let s = VisibilitySettings::REFERENCE;
        let mut index = index_with_observer()?;
        index.upsert(entity(1, 100, 101, 7)?);
        assert_eq!(index.query(&id(u32::MAX), s)?.entities().len(), 2);
        index.upsert(entity(1, 200, 200, 7)?);
        assert_eq!(index.query(&id(u32::MAX), s)?.entities().len(), 1);
        index.upsert(entity(1, 101, 100, 7)?);
        assert_eq!(index.query(&id(u32::MAX), s)?.entities().len(), 2);
        assert_eq!(
            index.get(&id(1)).map(|e| e.position),
            Some(pos(101, 100, 7)?)
        );
        assert!(index.remove(&id(1)).is_some());
        assert!(index.remove(&id(1)).is_none());
        assert_eq!(index.query(&id(u32::MAX), s)?.entities().len(), 1);
        Ok(())
    }

    fn snapshot(
        ids: impl Iterator<Item = u32>,
        revision: u64,
    ) -> Result<VisibilityQuery, InterestError> {
        let mut index = index_with_observer()?;
        for n in ids {
            let mut e = entity(n, 100, 100, 7)?;
            e.revision = revision;
            index.upsert(e);
        }
        index.query(&id(u32::MAX), VisibilitySettings::REFERENCE)
    }

    #[test]
    fn diff_reports_enter_leave_and_update() -> TestResult {
        let mut before = index_with_observer()?;
        before.upsert(entity(1, 100, 100, 7)?);
        before.upsert(entity(2, 100, 100, 7)?);
        before.upsert(entity(3, 100, 100, 7)?);
        let mut after = index_with_observer()?;
        after.upsert(entity(2, 100, 100, 7)?); // unchanged
        after.upsert(entity(3, 101, 100, 7)?); // moved
        after.upsert(entity(4, 100, 100, 7)?); // entered
        let mut bumped = entity(5, 100, 100, 7)?;
        before.upsert(bumped);
        bumped.revision = 9;
        after.upsert(bumped); // revision changed
        let s = VisibilitySettings::REFERENCE;
        let diff = diff_interest(
            &before.query(&id(u32::MAX), s)?,
            &after.query(&id(u32::MAX), s)?,
        );
        assert_eq!(
            diff,
            InterestDiff::Delta(vec![
                InterestChange::Leave(id(1)),
                InterestChange::Update(entity(3, 101, 100, 7)?),
                InterestChange::Enter(entity(4, 100, 100, 7)?),
                InterestChange::Update(bumped),
            ])
        );
        // Identical results produce an empty delta.
        let same = before.query(&id(u32::MAX), s)?;
        assert_eq!(diff_interest(&same, &same), InterestDiff::Delta(Vec::new()));
        Ok(())
    }

    #[test]
    fn diff_at_max_entries_is_a_delta_and_above_max_is_a_resync() -> TestResult {
        // Previous: others 0..255 (255 others + own). Current: others 128..383.
        // 128 leaves + 128 enters = 256 entries exactly.
        let previous = snapshot(0..255, 0)?;
        let at_max = snapshot(128..383, 0)?;
        match diff_interest(&previous, &at_max) {
            InterestDiff::Delta(changes) => assert_eq!(changes.len(), INTEREST_DELTA_ENTRIES_MAX),
            other => return Err(format!("expected a delta, got {other:?}").into()),
        }
        // Max + 1: one more identity changes revision, giving 257 entries.
        let mut over = snapshot(128..383, 0)?;
        if let Some(changed) = over.entities.iter_mut().find(|e| e.identity == id(200)) {
            changed.revision = 1;
        }
        match diff_interest(&previous, &over) {
            InterestDiff::Resync(snapshot) => assert_eq!(snapshot, over.entities),
            other => return Err(format!("expected a resync, got {other:?}").into()),
        }
        // A full replacement (255 leaves + 255 enters) is a resync.
        let replaced = snapshot(1000..1255, 0)?;
        assert!(matches!(
            diff_interest(&previous, &replaced),
            InterestDiff::Resync(_)
        ));
        // The own actor is in both results, so an empty-to-full change is 255 enters.
        let empty = snapshot(0..0, 0)?;
        let full = snapshot(0..255, 0)?;
        match diff_interest(&empty, &full) {
            InterestDiff::Delta(changes) => assert_eq!(changes.len(), 255),
            other => return Err(format!("expected a delta, got {other:?}").into()),
        }
        Ok(())
    }
}
