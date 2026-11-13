//! Select only admitted chosen-recipe profiles; retain original-source gaps as evidence.
use serde_json::Value;

pub(super) struct SelectedRecipe<'a> {
    pub recipe: Option<&'a Value>,
    pub complete: bool,
    pub original_source_gaps: bool,
}

impl SelectedRecipe<'_> {
    /// A selected, explicit Oteryn level replaces an unknown donor level. The
    /// original gap remains on the record; no other missing fact is resolved.
    pub(super) fn resolves_original_level(&self, gap: &Value) -> bool {
        self.original_source_gaps
            && self.complete
            && gap["code"] == "requirement_unknown"
            && gap["field"] == "min_level"
            && self.recipe.is_some_and(|recipe| {
                recipe["requirements"]["basis"] == "CHOSEN_OTERYN_APPROXIMATION"
                    && recipe["requirements"]["min_level"]
                        .as_u64()
                        .is_some_and(|level| u32::try_from(level).is_ok())
            })
    }
}

pub(super) fn select(definition: &Value) -> Result<Option<SelectedRecipe<'_>>, String> {
    if definition["definition_profile"] == "oteryn_authored_v1" {
        return Ok(Some(SelectedRecipe {
            recipe: definition.get("recipe").filter(|recipe| recipe.is_object()),
            complete: definition["data_complete"] == true,
            original_source_gaps: false,
        }));
    }
    let Some(chosen) = definition.get("oteryn_recipe") else {
        return Ok(None);
    };
    if chosen["profile"] != "chosen_source_completion_v1" {
        return Ok(None);
    }
    if chosen["payload"]["identity"] != definition["identity"] {
        return Err("imported chosen recipe owner differs from canonical Quest".into());
    }
    Ok(Some(SelectedRecipe {
        recipe: chosen["payload"]
            .get("recipe")
            .filter(|recipe| recipe.is_object()),
        complete: chosen["chosen_data_complete"] == true,
        // These describe the retained original, not missing selected Oteryn DATA.
        // They remain on the definition and do not imply original-source equivalence.
        original_source_gaps: true,
    }))
}
