//! ARMOR-MITIGATION-1: the worn armor total over the pinned production item profiles.
//! EQUIP-PROTECTION-1: the worn physical protection sum over the same profiles.

use super::*;

/// The pinned profiles with `armor` written into each named record's protection.
fn armored_input(armor: &[(&str, serde_json::Value)]) -> TestResult<NativeGameplayInput> {
    let protections: Vec<_> = armor
        .iter()
        .map(|(key, value)| {
            let protection = serde_json::json!({
                "state": "KNOWN",
                "value": {"armor": value, "resistances": {"state": "UNKNOWN"}},
            });
            (*key, protection)
        })
        .collect();
    protected_input(&protections)
}

/// The pinned profiles with each named record's whole protection replaced.
fn protected_input(protections: &[(&str, serde_json::Value)]) -> TestResult<NativeGameplayInput> {
    let mut input = input()?;
    let pin = input.item_profiles.as_mut().ok_or("profiles")?;
    let mut profiles: serde_json::Value = serde_json::from_slice(&pin.bytes)?;
    for (key, protection) in protections {
        let record = profiles["records"]
            .as_array_mut()
            .ok_or("records")?
            .iter_mut()
            .find(|r| r["production_definition"]["production_key"] == *key)
            .ok_or("record")?;
        record["semantics"]["protection"] = protection.clone();
    }
    pin.bytes = serde_json::to_vec(&profiles)?;
    use sha2::{Digest, Sha256};
    pin.sha256 = Sha256::digest(&pin.bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(input)
}

fn worn(content: &NativeGameplayState, items: &[(&str, u8)]) -> EquipmentSnapshot {
    let mut equipment = snapshot(content, items[0].0, items[0].1);
    equipment.items = items
        .iter()
        .flat_map(|(key, slot)| snapshot(content, key, *slot).items)
        .collect();
    equipment
}

const FEET: &str = "oteryn:item.tibia.i3549";
const AMULET: &str = "oteryn:item.tibia.i3081";
const SHIELD: &str = "oteryn:item.tibia.i3412";

#[test]
fn worn_armor_sums_own_slot_body_items_and_never_the_hands() -> TestResult {
    let known = |points: i32| serde_json::json!({"state": "KNOWN", "value": points});
    let input = armored_input(&[(FEET, known(2)), (AMULET, known(3)), (SHIELD, known(9))])?;
    let content = native(&input)?;
    assert_eq!(
        equipped_armor(&worn(&content, &[(FEET, 4), (AMULET, 7)]), &content),
        Some(5),
        "feet and necklace add"
    );
    assert_eq!(
        equipped_armor(&worn(&content, &[(FEET, 4), (SHIELD, 6)]), &content),
        Some(2),
        "a shield's armor is not `Player::getArmor`"
    );
    assert_eq!(
        equipped_armor(&worn(&content, &[(FEET, 8)]), &content),
        Some(0),
        "an item outside its own slot adds nothing"
    );
    let mut stale = worn(&content, &[(FEET, 4)]);
    stale.content_digest = [9; 32];
    assert_eq!(
        equipped_armor(&stale, &content),
        None,
        "another content refuses"
    );
    Ok(())
}

#[test]
fn unknown_or_negative_armor_adds_nothing() -> TestResult {
    let input = armored_input(&[
        (FEET, serde_json::json!({"state": "UNKNOWN"})),
        (AMULET, serde_json::json!({"state": "KNOWN", "value": -4})),
    ])?;
    let content = native(&input)?;
    assert_eq!(
        equipped_armor(&worn(&content, &[(FEET, 4), (AMULET, 7)]), &content),
        Some(0)
    );
    let pinned = native(&super::input()?)?;
    assert_eq!(
        equipped_armor(&worn(&pinned, &[(FEET, 4), (AMULET, 7)]), &pinned),
        Some(0),
        "the pinned profiles carry armor 0 or unknown"
    );
    Ok(())
}

/// A Known protection with no armor and these `(kind, percent)` resistances.
fn resisting(resistances: &[(&str, i64)]) -> serde_json::Value {
    let entries: Vec<_> = resistances
        .iter()
        .map(|(kind, percent)| {
            serde_json::json!({
                "kind": kind,
                "percent": {"state": "KNOWN", "value": {"denominator": 1, "numerator": percent}},
            })
        })
        .collect();
    serde_json::json!({
        "state": "KNOWN",
        "value": {
            "armor": {"state": "KNOWN", "value": 0},
            "resistances": {"state": "KNOWN", "value": entries},
        },
    })
}

/// Untimed feet: the pinned FEET and AMULET carry charges or a duration (EQUIP-0 §3.2).
const UNTIMED_FEET: &str = "oteryn:item.tibia.i6529";

#[test]
fn worn_physical_protection_counts_untimed_own_slot_items_without_field_protection() -> TestResult {
    let input = protected_input(&[
        (
            UNTIMED_FEET,
            resisting(&[("PHYSICAL", 4), ("FIRE_FIELD", 50)]),
        ),
        (AMULET, resisting(&[("PHYSICAL", 6)])),
    ])?;
    let content = native(&input)?;
    let sum = |items: &[(&str, u8)]| {
        equipment_effects(&worn(&content, items), &content)
            .as_ref()
            .and_then(physical_protection)
    };
    let four = Some(crate::content::ReferenceRationalPercent {
        numerator: 4,
        denominator: 1,
    });
    assert_eq!(
        sum(&[(UNTIMED_FEET, 4)]),
        four,
        "a field protection is not physical"
    );
    assert_eq!(
        sum(&[(UNTIMED_FEET, 4), (AMULET, 7)]),
        four,
        "a timed item adds nothing"
    );
    assert_eq!(
        sum(&[(UNTIMED_FEET, 8)]),
        None,
        "an item outside its own slot adds nothing"
    );
    let pinned = native(&super::input()?)?;
    assert_eq!(
        equipment_effects(&worn(&pinned, &[(UNTIMED_FEET, 4)]), &pinned)
            .as_ref()
            .and_then(physical_protection),
        None,
        "the pinned profiles carry no physical protection on these feet"
    );
    Ok(())
}
