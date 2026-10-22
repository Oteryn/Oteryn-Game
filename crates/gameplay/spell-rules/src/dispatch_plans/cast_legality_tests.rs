//! Reuse the existing authored-catalogue loader rather than synthesizing a Rune profile.
#![allow(clippy::unwrap_used)]
use super::*;
use crate::dispatch_plans::ordinary_combat::test_source;

#[test]
fn rune_range_boundary_keeps_native_floor_and_sight_precedence() {
    let spell = test_source("candidate:spell/rune/light_magic_missile_rune");
    let profile = spell.authored.as_ref().unwrap();
    assert_eq!(
        position_refusal(
            profile,
            (0, 0, -7),
            (7, 0, -7),
            Some(true),
            Some(false),
            Some(true)
        ),
        None
    );
    assert_eq!(
        position_refusal(
            profile,
            (0, 0, -7),
            (8, 0, -7),
            Some(true),
            Some(false),
            Some(true)
        ),
        Some(PositionRefusal::Range)
    );
    assert_eq!(
        position_refusal(
            profile,
            (0, 0, -7),
            (8, 0, -6),
            Some(true),
            Some(false),
            Some(true)
        ),
        Some(PositionRefusal::Spatial)
    );
    assert_eq!(
        position_refusal(
            profile,
            (0, 0, -7),
            (8, 0, -7),
            Some(false),
            Some(false),
            Some(true)
        ),
        Some(PositionRefusal::Spatial)
    );
}

#[test]
fn rune_known_solid_creature_exception_never_permits_unknown_solidity() {
    let spell = test_source("candidate:spell/rune/light_magic_missile_rune");
    let profile = spell.authored.as_ref().unwrap();
    assert_eq!(
        position_refusal(
            profile,
            (0, 0, -7),
            (1, 0, -7),
            Some(true),
            Some(true),
            Some(true)
        ),
        None
    );
    assert_eq!(
        position_refusal(
            profile,
            (0, 0, -7),
            (1, 0, -7),
            Some(true),
            None,
            Some(true)
        ),
        Some(PositionRefusal::Spatial)
    );
    assert_eq!(
        position_refusal(
            profile,
            (0, 0, -7),
            (1, 0, -7),
            Some(true),
            Some(true),
            Some(false)
        ),
        Some(PositionRefusal::Spatial)
    );
}

#[test]
fn protection_zone_requires_an_aggressive_authored_cast() {
    let spell = test_source("candidate:spell/rune/light_magic_missile_rune");
    let mut profile = spell.authored.as_ref().unwrap().as_ref().clone();
    assert!(protection_rejected(&profile, true));
    assert!(!protection_rejected(&profile, false));
    profile.header.targeting.aggressive = false;
    assert!(!protection_rejected(&profile, true));
}
