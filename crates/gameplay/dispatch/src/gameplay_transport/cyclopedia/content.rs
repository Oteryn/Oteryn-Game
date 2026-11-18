//! W12: discovery membership and POI candidates come from admitted native subarea masks.
use super::*;

pub(in crate::gameplay_transport) struct AdmittedArea {
    pub digest: [u8; 32],
    pub pool: Vec<Point>,
    pub area: u32,
    pub legacy_area: u32,
    pub subarea: Option<u32>,
    pub subareas: Vec<(u32, u32)>,
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
    let mask = table
        .masks
        .binary_search_by_key(&subarea, |mask| mask.subarea)
        .ok()?;
    let runs = || table.runs.iter().filter(|run| run.subarea == subarea);
    let mut examined = 0;
    let mut pool: Vec<_> = runs()
        .flat_map(|run| {
            (run.begin..run.end).map(move |x| Point {
                x: i32::from(x),
                y: i32::from(run.y),
                floor: i16::from(run.floor),
            })
        })
        .inspect(|_| examined += 1)
        .filter(|point| walkable(*point))
        .take(sampling.pool_limit)
        .collect();
    // Preserve saved POIs from the original prefix. Crystal discovery_system.lua:436-470
    // samples across the entire footprint, so also sample later mask runs within the
    // same probe budget as the walkable prefix. Skip unsampled tiles by run length.
    let remaining = table.masks[mask].admitted_tiles as usize - examined;
    if remaining > 0 {
        let stride = remaining.div_ceil(sampling.pool_limit);
        let mut sample = examined;
        let mut offset = 0;
        for run in runs() {
            let end = offset + usize::from(run.end - run.begin);
            while sample < end {
                let point = Point {
                    x: i32::from(run.begin) + (sample - offset) as i32,
                    y: i32::from(run.y),
                    floor: i16::from(run.floor),
                };
                if walkable(point) {
                    pool.push(point);
                }
                sample += stride;
            }
            offset = end;
        }
    }
    if pool.len() < sampling.generated_count {
        return None;
    }
    Some(AdmittedArea {
        digest: map.overlay.base().digest(),
        pool,
        area,
        legacy_area,
        subarea: Some(subarea),
        subareas,
    })
}
