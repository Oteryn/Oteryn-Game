//! WU26-14 (Winter Update 2026 test server): the target options of wave and beam spells on the
//! ordinary combat path, with the genuine sample spells.
#![allow(clippy::unwrap_used)]

use super::*;

fn source(key: &str) -> SpellDefinition {
    let catalog: Value = serde_json::from_slice(include_bytes!(
        "../../../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
    ))
    .unwrap();
    let row = catalog["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["bundle"]["spell"]["identity"]["key"] == key)
        .unwrap();
    oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle(
        &row["bundle"],
        &row["dependencies"],
    )
    .unwrap()
}

fn cast(target: SpellTarget, aim_at_target: bool) -> SpellCastIntent {
    SpellCastIntent {
        spell: std::num::NonZeroU32::MIN,
        target,
        aim_at_target,
    }
}

fn at(x: i32, y: i32) -> TilePosition {
    TilePosition { x, y, floor: 7 }
}

fn cursor(x: i32, y: i32) -> SpellTarget {
    SpellTarget::Position(oteryn_protocol_oteryn::actor_spell::SpellTargetPosition {
        x,
        y,
        floor: 7,
    })
}

fn wave_tiles(
    spell: &SpellDefinition,
    origin: TilePosition,
    facing: Direction,
) -> BTreeSet<TilePosition> {
    let a = ability(spell, None).unwrap();
    geometry(a, origin, facing).unwrap()
}

#[test]
fn wave_target_mode_hits_the_target_direction_while_facing_elsewhere() {
    let wave = source("candidate:spell/fire_wave");
    let caster = at(100, 100);
    let target = at(104, 101);
    // TARGET (`aim_at_target` with NONE) on a wave is an aimed cast.
    assert_eq!(
        aim(&wave, &cast(SpellTarget::None, true)),
        Ok(Some(Aim::Target))
    );
    // The character faces north, away from the target east of it.
    let facing = wave_tiles(&wave, at(100, 99), Direction::North);
    assert!(!facing.contains(&target));
    let (origin, d) = aimed_origin(caster, Some(Direction::North), Some(target)).unwrap();
    assert_eq!((origin, d), (at(101, 100), Direction::East));
    let aimed = wave_tiles(&wave, origin, d);
    assert!(aimed.contains(&target));
    assert!(!aimed.contains(&at(100, 99)));
}

#[test]
fn beam_target_mode_without_target_uses_facing() {
    let beam = source("candidate:spell/great_energy_beam");
    assert_eq!(
        aim(&beam, &cast(SpellTarget::None, true)),
        Ok(Some(Aim::Target))
    );
    let caster = at(100, 100);
    assert_eq!(
        aimed_origin(caster, Some(Direction::South), None),
        Ok((at(100, 101), Direction::South))
    );
    // No target and no facing is the facing cast's own refusal.
    assert_eq!(
        aimed_origin(caster, None, None),
        Err(SpellCastDisposition::TargetRequired)
    );
}

#[test]
fn wave_cursor_mode_keeps_caster_facing_and_breaks_ties_east_west() {
    let wave = source("candidate:spell/ice_wave");
    let caster = at(100, 100);
    assert_eq!(
        aim(&wave, &cast(cursor(97, 97), true)),
        Ok(Some(Aim::Cursor(at(97, 97))))
    );
    // A diagonal cursor goes west; north-facing stays the character's own facing.
    assert_eq!(
        aimed_origin(caster, Some(Direction::North), Some(at(97, 97))),
        Ok((at(99, 100), Direction::West))
    );
    assert_eq!(
        aimed_origin(caster, Some(Direction::North), Some(at(101, 104))),
        Ok((at(100, 101), Direction::South))
    );
    // The caster's own tile has no direction: the facing stays.
    assert_eq!(
        aimed_origin(caster, Some(Direction::North), Some(caster)),
        Ok((at(100, 99), Direction::North))
    );
}

#[test]
fn directional_position_target_keeps_caster_adjacent_origin() {
    let wave = source("candidate:spell/terra_wave");
    assert_eq!(
        aim(&wave, &cast(cursor(110, 100), true)),
        Ok(Some(Aim::Cursor(at(110, 100))))
    );
    let (origin, d) =
        aimed_origin(at(100, 100), Some(Direction::West), Some(at(110, 100))).unwrap();
    // The wave starts next to the caster, not at the cursor tile.
    assert_eq!((origin, d), (at(101, 100), Direction::East));
    assert!(wave_tiles(&wave, origin, d).contains(&at(102, 100)));
}

#[test]
fn invalid_target_option_for_non_wave_spell_is_refused() {
    let area = source("candidate:spell/berserk");
    for target in [SpellTarget::None, cursor(101, 100)] {
        assert_eq!(
            aim(&area, &cast(target, true)),
            Err(SpellCastDisposition::Rejected),
            "{target:?}"
        );
        assert_eq!(aim(&area, &cast(target, false)), Ok(None));
    }
    // A strike keeps the SPELL-D7 position cast and ignores the flag without a position.
    let strike = source("candidate:spell/flame_strike");
    assert_eq!(aim(&strike, &cast(cursor(101, 100), true)), Ok(None));
    assert_eq!(aim(&strike, &cast(SpellTarget::None, true)), Ok(None));
    // ATTACK_TARGET and an unaimed facing cast are unchanged for every spell.
    let wave = source("candidate:spell/fire_wave");
    for spell in [&area, &strike, &wave] {
        assert_eq!(aim(spell, &cast(SpellTarget::AttackTarget, true)), Ok(None));
        assert_eq!(aim(spell, &cast(SpellTarget::None, false)), Ok(None));
    }
}

#[test]
fn cursor_on_another_floor_is_target_illegal_not_the_facing() {
    let caster = MovementLocalPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    // Another floor is TARGET_ILLEGAL (`SPELL_AIM_MODE_CURSOR`), never a facing fallback.
    for floor in [6, 8] {
        assert_eq!(
            cursor_point(
                caster,
                TilePosition {
                    x: 101,
                    y: 100,
                    floor
                }
            ),
            Err(SpellCastDisposition::TargetIllegal),
            "{floor}"
        );
    }
    // On the caster's floor a cursor in view aims; one outside the view keeps the facing.
    assert_eq!(cursor_point(caster, at(97, 97)), Ok(Some(at(97, 97))));
    assert_eq!(cursor_point(caster, at(130, 100)), Ok(None));
    assert_eq!(
        aimed_origin(
            at(100, 100),
            Some(Direction::North),
            cursor_point(caster, at(130, 100)).unwrap()
        ),
        Ok((at(100, 99), Direction::North))
    );
}
