//! Source-qualified native spell data, without runtime planning.
use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::OnceLock;

const PROFILE_JSON: &str = include_str!(
    "../../../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
);
const REVISION: &str = "spell-p2-r20";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

pub fn profiles() -> Result<&'static Vec<Value>, Error> {
    static PROFILES: OnceLock<Result<Vec<Value>, Error>> = OnceLock::new();
    PROFILES
        .get_or_init(|| {
            let document: Value =
                serde_json::from_str(PROFILE_JSON).map_err(|e| Error(e.to_string()))?;
            if document["revision"] != REVISION {
                return Err(Error("unqualified native profile revision".into()));
            }
            let profiles = document["profiles"]
                .as_array()
                .ok_or_else(|| Error("missing native profiles".into()))?;
            if profiles.len() != 67 {
                return Err(Error("incomplete native profile coverage".into()));
            }
            let mut identities = BTreeSet::new();
            for profile in profiles {
                let spell = &profile["spell"];
                if profile["name"] != spell["name"]
                    || profile["carrier"] != spell["carrier"]
                    || profile["execution"] != spell["execution"]
                    || spell["identity"]["revision"] != REVISION
                    || !identities
                        .insert((profile["carrier"].to_string(), profile["name"].to_string()))
                {
                    return Err(Error("inconsistent or duplicate native profile".into()));
                }
            }
            Ok(profiles.clone())
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// Construction is private; altered parameters, costs, targeting and dependencies
/// cannot reach any planner through this reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledNativeSpell {
    spell: Value,
    dependencies: Value,
}

pub fn spell_from_bundle(
    bundle: &Value,
    dependencies: &Value,
) -> Result<CompiledNativeSpell, Error> {
    let object = bundle
        .as_object()
        .ok_or_else(|| Error("expected spell bundle".into()))?;
    if object.len() != 1 || !object.contains_key("spell") {
        return Err(Error("unexpected spell bundle properties".into()));
    }
    let spell = &bundle["spell"];
    let profile = profiles()?
        .iter()
        .find(|profile| profile["name"] == spell["name"] && profile["carrier"] == spell["carrier"])
        .ok_or_else(|| Error("spell has no qualified native profile".into()))?;
    if spell != &profile["spell"] || dependencies != &profile["dependencies"] {
        return Err(Error(
            "native spell differs from its qualified profile".into(),
        ));
    }
    Ok(CompiledNativeSpell {
        spell: spell.clone(),
        dependencies: dependencies.clone(),
    })
}

impl CompiledNativeSpell {
    /// The complete qualified cast header remains available to the cast owner for
    /// its common checks and the single cost/cooldown commitment.
    pub fn dependencies(&self) -> &Value {
        &self.dependencies
    }
    pub fn spell(&self) -> &Value {
        &self.spell
    }
}

#[cfg(test)]
mod tests {
    // Panicking assertions are confined to regression tests.
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    #[test]
    fn all_67_profiles_load_and_semantic_edits_are_refused() {
        for profile in profiles().unwrap() {
            let bundle = json!({"spell": profile["spell"]});
            let dependencies = &profile["dependencies"];
            assert!(
                spell_from_bundle(&bundle, dependencies).is_ok(),
                "{}",
                profile["name"]
            );
            for altered in ["costs", "targeting", "execution", "requirements"] {
                let mut changed = bundle.clone();
                changed["spell"][altered]["unqualified"] = json!(true);
                assert!(spell_from_bundle(&changed, dependencies).is_err());
            }
            let mut changed = dependencies.clone();
            changed["unqualified"] = json!(true);
            assert!(spell_from_bundle(&bundle, &changed).is_err());
        }
    }
}
