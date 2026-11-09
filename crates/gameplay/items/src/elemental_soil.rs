//! Crystal 201189f8 elemental_spheres/soils1.lua and items.xml:20982-20994.
//! Energy soil creates a combat field and is not a terrain transformation.
use crate::fishing::GatheringAttempt;

/// Soil, inactive target, charged target. Each charged object decays in 120 seconds.
pub const CHARGES: [(u16, u16, u16); 3] = [(940, 7742, 7746), (941, 7743, 7745), (944, 7741, 7744)];

pub fn charge(soil: u16, target: u16) -> Option<GatheringAttempt> {
    let (_, _, after) = CHARGES
        .iter()
        .find(|(item, before, _)| *item == soil && *before == target)?;
    Some(GatheringAttempt {
        after: Some(*after),
        yield_item: None,
        consumes_bait: true,
    })
}

pub fn decays(charged: u16) -> Vec<(u32, i64)> {
    CHARGES
        .iter()
        .find(|(_, _, after)| *after == charged)
        .map(|(_, before, _)| vec![(u32::from(*before), 120_000)])
        .unwrap_or_default()
}
