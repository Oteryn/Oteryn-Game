//! W12: discovery membership and POI candidates come from admitted native subarea masks.
use super::*;
use oteryn_world_bundle::subarea::Table;
use std::collections::BTreeSet;

pub(in crate::gameplay_transport) struct AdmittedArea {
    pub digest: [u8; 32],
    pub pool: Vec<Point>,
    pub area: u32,
    pub legacy_area: u32,
    pub main_areas: BTreeSet<u32>,
    pub subarea: Option<u32>,
    pub subareas: Vec<(u32, u32)>,
}
pub(super) fn main_areas(table: &Table) -> BTreeSet<u32> {
    table
        .masks
        .iter()
        .filter(|mask| mask.admitted_tiles > 0)
        .filter_map(|mask| {
            let index = table
                .areas
                .binary_search_by_key(&mask.subarea, |area| area.id)
                .ok()?;
            table.areas[index].parent
        })
        .collect()
}

pub(super) fn admit(owner: &ComposedFreshAdmission<'_, '_, '_>) -> Option<AdmittedArea> {
    let map = owner.bundle_map()?;
    admit_at(
        owner,
        Point {
            x: i32::from(map.entry_start.x),
            y: i32::from(map.entry_start.y),
            floor: i16::from(map.entry_start.floor),
        },
    )
}
pub(super) fn admit_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    start: Point,
) -> Option<AdmittedArea> {
    let sampling = BasicArea::embedded()?;
    let native = owner.active_generation?.native_gameplay()?;
    let index = native.item_index()?;
    for key in [
        "oteryn:item.tibia.i2854",
        "oteryn:item.tibia.i3031",
        "oteryn:item.tibia.i3035",
        "oteryn:item.tibia.i3043",
    ] {
        index.revision(key)?;
    }
    let map = owner.bundle_map()?;
    let table = &map.overlay.base().subareas;
    let (area, subarea) = table.resolve(
        u16::try_from(start.x).ok()?,
        u16::try_from(start.y).ok()?,
        i8::try_from(start.floor).ok()?,
    )?;
    let entry = map.entry_start;
    let legacy_area = table.resolve(entry.x, entry.y, entry.floor)?.0;
    let walkable = |point: Point| {
        let (Ok(x), Ok(y), Ok(floor)) = (
            u16::try_from(point.x),
            u16::try_from(point.y),
            i8::try_from(point.floor),
        ) else {
            return false;
        };
        if map
            .facts
            .doors()
            .at(crate::map::overlay::TilePos { x, y, floor })
        {
            return false;
        }
        let cells = owner.movement_cells();
        cells.index().lookup(
            cells.scope(),
            LogicalCell {
                x: point.x,
                y: point.y,
                z: i32::from(-point.floor),
            },
        ) == Ok(CollisionClass::Walkable)
    };
    if !walkable(start) {
        return None;
    }
    let subareas: Vec<_> = table
        .areas
        .iter()
        .filter_map(|a| a.parent.map(|parent| (parent, a.id)))
        .collect();
    let mut pool = Vec::with_capacity(sampling.pool_limit);
    let mut visible = Vec::with_capacity(sampling.generated_count);
    'candidates: for run in table.runs.iter().filter(|run| run.subarea == subarea) {
        for x in run.begin..run.end {
            let point = Point {
                x: i32::from(x),
                y: i32::from(run.y),
                floor: i16::from(run.floor),
            };
            if point.in_window(start) {
                // Small subareas still need to admit progress reads and finds.
                if visible.len() < sampling.generated_count && walkable(point) {
                    visible.push(point);
                }
            } else if walkable(point) {
                // Crystal discovery_system.lua:458–488 checks visibility before
                // counting candidates. Bound the hidden pool across the subarea.
                pool.push(point);
                if pool.len() == sampling.pool_limit {
                    break 'candidates;
                }
            }
        }
    }
    pool.extend(
        visible
            .into_iter()
            .take(sampling.generated_count.saturating_sub(pool.len())),
    );
    if pool.len() < sampling.generated_count {
        return None;
    }
    Some(AdmittedArea {
        digest: map.overlay.base().digest(),
        pool,
        area,
        legacy_area,
        main_areas: main_areas(table),
        subarea: Some(subarea),
        subareas,
    })
}
