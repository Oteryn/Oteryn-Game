use super::*;

#[test]
fn item_conversion_weighted_ceiling_24_accepted_25_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../../content/craft/recipes.json"
    ))?;
    let mut recipe: ProjectV2ItemConversion = serde_json::from_value(
        catalogue["records"]
            .as_array()
            .ok_or("records")?
            .iter()
            .find(|r| r["identity"]["key"] == "oteryn:item-conversion.corpse-mechanical")
            .ok_or("mechanical row")?["recipe"]
            .clone(),
    )?;
    let ProjectV2ConversionOutputs::Weighted { alternatives } = &mut recipe.outputs else {
        return Err("weighted".into());
    };
    assert_eq!(alternatives.len(), 19);
    alternatives.resize(24, alternatives[0].clone());
    recipe.validate_shape(crate::content::native_entry_first_slice_limits().project)?;
    let ProjectV2ConversionOutputs::Weighted { alternatives } = &mut recipe.outputs else {
        return Err("weighted".into());
    };
    alternatives.push(alternatives[0].clone());
    assert!(
        recipe
            .validate_shape(crate::content::native_entry_first_slice_limits().project)
            .is_err()
    );
    Ok(())
}

#[test]
fn item_conversion_capture_target_stack_is_explicit_and_target_only()
-> Result<(), Box<dyn std::error::Error>> {
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../../content/craft/recipes.json"
    ))?;
    let mut value = catalogue["records"]
        .as_array()
        .ok_or("records")?
        .iter()
        .find(|r| r["identity"]["key"] == "oteryn:item-conversion.sugar-oat")
        .ok_or("carried row")?["recipe"]
        .clone();
    value["operands"][1]["capture_target_stack"] = true.into();
    let mut recipe: ProjectV2ItemConversion = serde_json::from_value(value)?;
    let limits = crate::content::native_entry_first_slice_limits().project;
    recipe.validate_shape(limits)?;
    recipe.operands[0].capture_target_stack = true;
    assert!(recipe.validate_shape(limits).is_err());
    recipe.operands[0].capture_target_stack = false;
    recipe.operands[1].count = 2;
    assert!(recipe.validate_shape(limits).is_err());
    recipe.operands[1].count = 1;
    recipe.operands[1].effect = ProjectV2ConversionEffect::Keep;
    assert!(recipe.validate_shape(limits).is_err());
    Ok(())
}
