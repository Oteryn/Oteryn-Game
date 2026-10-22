//! Authored spatial checks shared by native casting and diagnose-only USE reporting.
use super::geometry::TilePosition;
use oteryn_gameplay_content::content::spell_data::executable_catalog::OperationalProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionRefusal {
    Spatial,
    Range,
}

pub fn protection_rejected(profile: &OperationalProfile, in_protection_zone: bool) -> bool {
    profile.header.targeting.aggressive && in_protection_zone
}

/// Preserve the native operational validator's floor, sight, reach and Rune blocking order.
pub fn position_refusal(
    profile: &OperationalProfile,
    caster: TilePosition,
    target: TilePosition,
    sight_clear: Option<bool>,
    target_solid: Option<bool>,
    target_creature: Option<bool>,
) -> Option<PositionRefusal> {
    let header = &profile.header;
    let reach = reach_refusal(profile, caster, target);
    if reach == Some(PositionRefusal::Spatial) {
        return reach;
    }
    if header.targeting.block_walls && sight_clear != Some(true) {
        return Some(PositionRefusal::Spatial);
    }
    if reach.is_some() {
        return reach;
    }
    if let Some(rune) = &header.rune {
        // A known solid cell permits its independently resolved creature, as in
        // Rune::playerRuneSpellCheck. Unknown solidity gets no exception.
        if rune.blocking.solid
            && !(target_solid == Some(false)
                || (target_solid == Some(true) && target_creature == Some(true)))
        {
            return Some(PositionRefusal::Spatial);
        }
        if rune.blocking.creature && target_creature != Some(false) {
            return Some(PositionRefusal::Spatial);
        }
    }
    None
}

/// Diagnose floor/reach before tile IO; native validation still checks sight before reach.
pub fn reach_refusal(
    profile: &OperationalProfile,
    caster: TilePosition,
    target: TilePosition,
) -> Option<PositionRefusal> {
    let header = &profile.header;
    if header.targeting.check_floor && target.2 != caster.2 {
        return Some(PositionRefusal::Spatial);
    }
    let distance = target.0.abs_diff(caster.0).max(target.1.abs_diff(caster.1));
    if header
        .targeting
        .range_tiles
        .is_some_and(|range| distance > range)
        || header
            .rune
            .as_ref()
            .is_some_and(|rune| !rune.allow_far_use && distance > 1)
    {
        return Some(PositionRefusal::Range);
    }
    None
}

#[cfg(test)]
#[path = "cast_legality_tests.rs"]
mod tests;
