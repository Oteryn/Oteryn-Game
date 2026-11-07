//! PARITY-EQUIP-MODIFIERS-1: the worn critical hit and leech over the pinned production profiles.

use super::*;

const BACKPACK: &str = "oteryn:item.tibia.i2854";

/// One Known rational percent modifier of `kind`.
fn percent(kind: &str, numerator: i64, denominator: u64) -> serde_json::Value {
    serde_json::json!({
        "kind": kind,
        "parameter": {"state": "KNOWN", "value": {"kind": "RATIONAL_PERCENT", "value": {"numerator": numerator, "denominator": denominator}}},
        "evaluation_phase": {"state": "UNKNOWN"},
        "priority": {"state": "UNKNOWN"},
        "target_domain": {"state": "UNKNOWN"},
    })
}

#[test]
fn worn_offence_sums_own_slot_items_in_hundredths_of_a_percent() -> TestResult {
    let input = fixture_input(
        serde_json::json!([
            percent("CRITICAL_HIT_CHANCE", 5, 1),
            percent("CRITICAL_HIT_DAMAGE", 25, 1),
            percent("LIFE_LEECH_AMOUNT", 18, 1),
            percent("LIFE_LEECH_CHANCE", 100, 1),
            percent("MANA_LEECH_AMOUNT", 11, 6),
        ]),
        Some(0),
    )?;
    let content = native(&input)?;
    assert_eq!(
        equipped_offence(&snapshot(&content, BACKPACK, 9), &content),
        Some(EquippedOffence {
            critical_chance: 500,
            critical_damage: 2500,
            life_leech: 1800,
            mana_leech: 183,
            ..EquippedOffence::default()
        }),
        "a leech chance is never read (Canary `Game::applyLifeLeech`)"
    );
    assert_eq!(
        equipped_offence(&snapshot(&content, BACKPACK, 4), &content),
        Some(EquippedOffence::default()),
        "an item outside its own slot adds nothing"
    );
    let mut stale = snapshot(&content, BACKPACK, 9);
    stale.content_digest = [9; 32];
    assert_eq!(
        equipped_offence(&stale, &content),
        None,
        "another content refuses"
    );
    let pinned = native(&super::input()?)?;
    assert_eq!(
        equipped_offence(&snapshot(&pinned, BACKPACK, 9), &pinned),
        Some(EquippedOffence::default()),
        "the pinned profiles carry no critical hit or leech"
    );
    Ok(())
}

#[test]
fn worn_offence_clamps_to_the_skill_range() -> TestResult {
    let input = fixture_input(
        serde_json::json!([
            percent("CRITICAL_HIT_CHANCE", 1000, 1),
            percent("LIFE_LEECH_AMOUNT", -5, 1),
        ]),
        Some(0),
    )?;
    let content = native(&input)?;
    let offence = equipped_offence(&snapshot(&content, BACKPACK, 9), &content).ok_or("offence")?;
    assert_eq!((offence.critical_chance, offence.life_leech), (u16::MAX, 0));
    Ok(())
}

#[test]
fn worn_offence_refuses_a_tier_without_a_current_forge_profile() -> TestResult {
    let content = native(&super::input()?)?;
    let mut worn = snapshot(&content, BACKPACK, 9);
    worn.items[0].tier = 1;
    assert_eq!(equipped_offence(&worn, &content), None);
    Ok(())
}

#[test]
fn worn_eqmod2_reads_cleave_reflection_and_paired_perfect_shot_attributes() -> TestResult {
    let points = |kind: &str, shape: &str, value: u32| {
        serde_json::json!({
            "kind": kind,
            "parameter": {"state":"KNOWN", "value":{"kind":shape,"value":value}},
            "evaluation_phase":{"state":"UNKNOWN"}, "priority":{"state":"UNKNOWN"}, "target_domain":{"state":"UNKNOWN"}
        })
    };
    let input = fixture_input(
        serde_json::json!([
            percent("CLEAVE_PERCENT", 7, 2),
            points("PERFECT_SHOT_DAMAGE", "SIGNED_POINTS", 20),
            points("PERFECT_SHOT_RANGE", "CELLS", 3),
            points("REFLECT_DAMAGE", "SIGNED_POINTS", 5),
            percent("REFLECT_PERCENT_ALL", 10, 1)
        ]),
        Some(0),
    )?;
    let content = native(&input)?;
    let offence = equipped_offence(&snapshot(&content, BACKPACK, 9), &content).ok_or("offence")?;
    assert_eq!(
        (
            offence.cleave,
            offence.reflect_percent,
            offence.reflect_flat
        ),
        (350, 1000, 5)
    );
    assert_eq!(offence.perfect_damage(3, 3), 20);
    assert_eq!(offence.reflected(100, true, 1, 50_000), 15);
    assert_eq!(offence.reflected(100, false, 8, 50_000), 10);
    Ok(())
}
