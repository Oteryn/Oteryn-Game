//! Native boot admission for closed authored binding documents. This does not qualify a quest.
use super::{Bindings, MAX_BINDING_BYTES, MAX_BOUND_QUESTS};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

include!(concat!(env!("OUT_DIR"), "/authored_binding_documents.rs"));
const INDEX: &str = include_str!("../../../../../../content/quests/definitions/index.json");

impl Bindings {
    /// Combine a complete trusted binding snapshot; names never imply another source document.
    pub fn parse_documents(documents: &[&str]) -> Result<Self, String> {
        if documents.len() > MAX_BOUND_QUESTS
            || documents
                .iter()
                .try_fold(0usize, |sum, document| sum.checked_add(document.len()))
                .is_none_or(|bytes| bytes > MAX_BINDING_BYTES)
        {
            return Err("authored binding document set exceeds limits".into());
        }
        let mut combined = Self {
            schema: 1,
            quests: Vec::new(),
        };
        for document in documents {
            let parsed = Self::parse(document.as_bytes())?;
            combined.quests.extend(parsed.quests);
        }
        combined.validate()?;
        Ok(combined)
    }

    /// Verify the exact authored recipe in the same compiled canonical Quest population.
    pub fn verify_embedded_recipe_evidence(&self) -> Result<(), String> {
        RecipeEvidence::embedded()?.verify(self)
    }
}

pub(crate) fn load() -> Result<Option<Bindings>, String> {
    if AUTHORED_BINDING_DOCUMENTS.is_empty() {
        return Ok(None);
    }
    let documents: Vec<_> = AUTHORED_BINDING_DOCUMENTS
        .iter()
        .map(|(_, document)| *document)
        .collect();
    let bindings = Bindings::parse_documents(&documents)?;
    if bindings.quests.is_empty() {
        return Err("registered authored bindings contain no quests".into());
    }
    bindings.verify_embedded_recipe_evidence()?;
    Ok(Some(bindings))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RecipeEvidence {
    recipes: BTreeMap<String, Recipe>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Recipe {
    digest: Option<String>,
    complete: bool,
    data_holds: bool,
    claim_only_pending: bool,
    minimum_level: Option<u32>,
    requires_premium: Option<bool>,
    prerequisites: Option<Vec<String>>,
}

impl RecipeEvidence {
    pub(crate) fn embedded() -> Result<Self, String> {
        Self::from_documents(INDEX, CANONICAL_QUEST_SHARDS)
    }

    fn from_documents(index: &str, shards: &[(&str, &str)]) -> Result<Self, String> {
        let index: Value = serde_json::from_str(index).map_err(|error| error.to_string())?;
        if index["schema"] != "OTERYN_FAMILY_INDEX/v1" || index["family"] != "Quest" {
            return Err("canonical Quest index schema/family".into());
        }
        let listed = index["shards"]
            .as_array()
            .ok_or("canonical Quest shard list")?;
        let listed: Vec<_> = listed
            .iter()
            .map(|path| path.as_str().ok_or("canonical Quest shard path"))
            .collect::<Result<_, _>>()?;
        let expected: BTreeSet<_> = listed.iter().copied().collect();
        let actual: BTreeSet<_> = shards.iter().map(|(path, _)| *path).collect();
        if listed.len() != expected.len()
            || actual.len() != shards.len()
            || expected != actual
            || shards.len() > 64
        {
            return Err("canonical Quest shard membership differs from index".into());
        }
        let mut definitions = BTreeMap::new();
        for (_, source) in shards {
            let shard: Value = serde_json::from_str(source).map_err(|error| error.to_string())?;
            if shard["schema"] != "OTERYN_QUEST_TREE_SHARD/v1" || shard["family"] != "Quest" {
                return Err("canonical Quest shard schema/family".into());
            }
            for record in shard["records"]
                .as_array()
                .ok_or("canonical Quest records")?
            {
                let definition = &record["definition"];
                let key = definition["identity"]["key"]
                    .as_str()
                    .ok_or("canonical Quest identity")?;
                if definitions
                    .insert(key.to_owned(), definition.clone())
                    .is_some()
                    || definitions.len() > 4096
                {
                    return Err("canonical Quest identities are duplicate or exceed limits".into());
                }
            }
        }
        if index["record_count"].as_u64() != u64::try_from(definitions.len()).ok() {
            return Err("canonical Quest population differs from index".into());
        }
        let mut recipes = BTreeMap::new();
        for (key, definition) in definitions {
            let Some(selected) = super::imported_recipe::select(&definition)? else {
                continue;
            };
            let missing = definition["missing_data"]
                .as_array()
                .ok_or("canonical Quest missing-data codes")?;
            let data_holds = missing.iter().any(|entry| {
                !matches!(
                    entry["code"].as_str(),
                    Some(
                        "quest_native_lowering_missing"
                            | "authored_trigger_and_delivery_bindings_missing"
                    )
                ) && !(selected.original_source_gaps
                    && entry["code"].as_str() == Some("reported_source_gap"))
                    && !selected.resolves_original_level(entry)
            });
            // A missing legacy delivery owner must not prevent a safe first step.
            // Every other data hold remains authoritative, as do nontrivial actor gates.
            let claim_only_pending = missing
                .iter()
                .any(|entry| entry["code"] == "claim_native_lowering_missing")
                && missing.iter().all(|entry| {
                    matches!(
                        entry["code"].as_str(),
                        Some(
                            "claim_native_lowering_missing"
                                | "quest_native_lowering_missing"
                                | "authored_trigger_and_delivery_bindings_missing"
                        )
                    ) || (selected.original_source_gaps && entry["code"] == "reported_source_gap")
                });
            let minimum_level = selected
                .recipe
                .and_then(|recipe| recipe["requirements"]["min_level"].as_u64())
                .and_then(|minimum| u32::try_from(minimum).ok());
            let requires_premium = selected
                .recipe
                .and_then(|recipe| recipe["requirements"]["premium"].as_bool());
            let prerequisites = selected
                .recipe
                .and_then(|recipe| recipe["requirements"]["prerequisites"].as_array())
                .and_then(|values| {
                    values
                        .iter()
                        .map(|value| value.as_str().map(str::to_owned))
                        .collect::<Option<Vec<_>>>()
                });
            let digest = selected
                .recipe
                .map(|recipe| {
                    let bytes = serde_json::to_vec(recipe).map_err(|error| error.to_string())?;
                    Ok::<String, String>(
                        Sha256::digest(bytes)
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect(),
                    )
                })
                .transpose()?;
            recipes.insert(
                key,
                Recipe {
                    digest,
                    complete: selected.complete,
                    data_holds,
                    claim_only_pending,
                    minimum_level,
                    requires_premium,
                    prerequisites,
                },
            );
        }
        Ok(Self { recipes })
    }

    pub(crate) fn verify(&self, bindings: &Bindings) -> Result<(), String> {
        bindings.validate()?;
        for quest in &bindings.quests {
            let recipe = self.recipes.get(&quest.quest).ok_or_else(|| {
                format!(
                    "authored binding has no canonical chosen recipe: {}",
                    quest.quest
                )
            })?;
            if !recipe.complete || recipe.digest.is_none() {
                return Err(format!(
                    "authored binding requires a complete chosen recipe: {}",
                    quest.quest
                ));
            }
            let native_observations = quest.stages.iter().all(|stage| {
                matches!(
                    stage.trigger,
                    super::Trigger::TerrainUse { .. }
                        | super::Trigger::DoorUse { .. }
                        | super::Trigger::TaskBoardUse { .. }
                        | super::Trigger::Collect { .. }
                        | super::Trigger::Kill { .. }
                )
            });
            let declared_prerequisites: Vec<_> = quest
                .admission
                .as_ref()
                .into_iter()
                .flat_map(|gate| &gate.prerequisites)
                .map(|required| required.requirement.clone())
                .collect();
            let admission_matches = recipe.prerequisites.as_ref() == Some(&declared_prerequisites)
                && recipe.requires_premium
                    == Some(quest.admission.as_ref().is_some_and(|gate| gate.premium))
                && recipe.minimum_level
                    == Some(
                        quest
                            .admission
                            .as_ref()
                            .map_or(0, |gate| gate.minimum_level),
                    );
            if (native_observations || quest.admission.is_some()) && !admission_matches {
                return Err(format!(
                    "authored Quest requires its exact native actor admission: {}",
                    quest.quest
                ));
            }
            let pure_progress =
                native_observations && quest.stages.iter().all(|stage| !stage.completes);
            if recipe.data_holds
                && !(recipe.claim_only_pending && admission_matches && pure_progress)
            {
                return Err(format!(
                    "authored Quest has unresolved data holds: {}",
                    quest.quest
                ));
            }
            if recipe.digest.as_deref() != Some(quest.recipe_sha256.as_str()) {
                return Err(format!(
                    "authored recipe digest differs from canonical Quest: {}",
                    quest.quest
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "embedded_tests.rs"]
mod tests;
