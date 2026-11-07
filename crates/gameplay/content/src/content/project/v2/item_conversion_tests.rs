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
