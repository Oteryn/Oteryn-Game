//! C5 authored map transitions must retain the pinned XML return timer.

use super::*;

pub(super) fn validate(recipe: &ProjectV2ItemConversion) -> Result<(), ProjectError> {
    let Some(opened) = &recipe.opened_target else {
        return Ok(());
    };
    let facts: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../../../content/craft/map-facts.json"
    ))
    .map_err(|e| ProjectError::InvalidJson(e.to_string()))?;
    let target = match &recipe.target {
        ProjectV2ConversionTarget::MapItem { definition }
        | ProjectV2ConversionTarget::MapObject { definition } => definition,
        _ => {
            return Err(ProjectError::InvalidProject(
                "item_conversion digging target",
            ));
        }
    };
    let expected = facts["opened_forms"].as_array().and_then(|forms| {
        forms.iter().find(|form| {
            form["item_id"]
                .as_u64()
                .is_some_and(|id| opened.key == format!("oteryn:item.tibia.i{id}"))
        })
    });
    if !expected.is_some_and(|form| {
        form["duration_ms"].as_u64() == recipe.revert_ms.map(u64::from)
            && form["decay_to"]
                .as_u64()
                .is_some_and(|id| target.key == format!("oteryn:item.tibia.i{id}"))
    }) {
        return Err(ProjectError::InvalidProject(
            "item_conversion digging return differs from pinned XML",
        ));
    }
    Ok(())
}
