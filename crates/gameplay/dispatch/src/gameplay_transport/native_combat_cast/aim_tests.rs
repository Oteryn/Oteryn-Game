//! WU26-14 (Winter Update 2026 test server): the target options of the native wave and beam
//! spells take the ordinary path's rule and resolver, with the genuine sample spells.
#![allow(clippy::unwrap_used)]

use super::super::ordinary_combat::{Aim, aimed_origin};
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

#[test]
fn native_waves_and_beams_accept_target_and_cursor_aim() {
    for (key, native) in [
        ("candidate:spell/energy_beam", "wheel_combat"),
        ("candidate:spell/energy_wave", "wheel_combat"),
        ("candidate:spell/front_sweep", "wheel_combat"),
        ("candidate:spell/great_energy_beam", "wheel_combat"),
        ("candidate:spell/strong_ice_wave", "wheel_combat"),
        ("candidate:spell/balanced_brawl", "monster_ai_override"),
    ] {
        let spell = source(key);
        assert_eq!(
            native_aim(native, &spell, &cast(SpellTarget::None, true)),
            Ok(Some(Aim::Target)),
            "{key}"
        );
        assert_eq!(
            native_aim(native, &spell, &cast(cursor(104, 101), true)),
            Ok(Some(Aim::Cursor(at(104, 101)))),
            "{key}"
        );
        // A facing cast and ATTACK_TARGET keep their native handling.
        assert_eq!(
            native_aim(native, &spell, &cast(SpellTarget::None, false)),
            Ok(None),
            "{key}"
        );
        assert_eq!(
            native_aim(native, &spell, &cast(SpellTarget::AttackTarget, true)),
            Ok(None),
            "{key}"
        );
    }
    // The resolver they share: a target east of a north-facing caster turns the wave east from
    // the caster-adjacent tile, and without a point the facing stays.
    assert_eq!(
        aimed_origin(at(100, 100), Some(Direction::North), Some(at(104, 101))),
        Ok((at(101, 100), Direction::East))
    );
    assert_eq!(
        aimed_origin(at(100, 100), Some(Direction::North), None),
        Ok((at(100, 99), Direction::North))
    );
}

#[test]
fn native_aim_keeps_delayed_strikes_and_refuses_non_directional_spells() {
    let strike = source("candidate:spell/death_echo");
    for target in [
        SpellTarget::None,
        cursor(101, 100),
        SpellTarget::AttackTarget,
    ] {
        assert_eq!(
            native_aim("delayed_strike", &strike, &cast(target, true)),
            Ok(None),
            "{target:?}"
        );
    }
    let mend = source("candidate:spell/mass_spirit_mend");
    for target in [SpellTarget::None, cursor(101, 100)] {
        assert_eq!(
            native_aim("mass_spirit_mend", &mend, &cast(target, true)),
            Err(SpellCastDisposition::Rejected),
            "{target:?}"
        );
    }
    assert_eq!(
        native_aim("mass_spirit_mend", &mend, &cast(SpellTarget::None, false)),
        Ok(None)
    );
}
