use super::*;
#[test]
fn m10_effects_are_part_of_the_source_payment_and_interview_plans() -> Result<(), String> {
    let callbacks = load()?;
    let axe = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/qnpcfx-oldrak-hallowed-axe")
        .ok_or("Oldrak binding")?;
    let value = serde_json::to_value(axe).map_err(|e| e.to_string())?;
    assert_eq!(value["magic_effects"][0]["effect"], "CONST_ME_YELLOWENERGY");
    assert_eq!(value["magic_effects"][0]["target"], "npc");
    assert_eq!(axe.fee.as_ref().map(|f| f.gold), Some(1000));
    assert!(
        axe.is_paid_exchange(),
        "Oldrak can refresh the axe again for a fresh payment"
    );
    assert_eq!(
        callbacks
            .iter()
            .filter(
                |c| c.key.starts_with("oteryn:quest-reward/qnpcfx-inquisition-")
                    && c.key.ends_with("_guard")
            )
            .count(),
        5
    );
    Ok(())
}
