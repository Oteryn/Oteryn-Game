//! ARMOR-MITIGATION-1: the worn armor total over the pinned production item profiles.

use super::*;

/// The pinned profiles with `armor` written into each named record's protection.
fn armored_input(armor: &[(&str, serde_json::Value)]) -> TestResult<NativeGameplayInput> {
    let mut input = input()?;
    let pin = input.item_profiles.as_mut().ok_or("profiles")?;
    let mut profiles: serde_json::Value = serde_json::from_slice(&pin.bytes)?;
    for (key, value) in armor {
        let record = profiles["records"]
            .as_array_mut()
            .ok_or("records")?
            .iter_mut()
            .find(|r| r["production_definition"]["production_key"] == *key)
            .ok_or("record")?;
        record["semantics"]["protection"] = serde_json::json!({
            "state": "KNOWN",
            "value": {"armor": value, "resistances": {"state": "UNKNOWN"}},
        });
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
