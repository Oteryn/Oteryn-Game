//! Closed, reproducible selection of a Wiki-identified leaf for an owned once recipe.
//! The ordinary source selector, native Item admission and once MINT owner remain authoritative.
use super::*;
use sha2::{Digest, Sha256};

pub(super) const PROFILE: &str = "native_selected_leaf_v1";
const SCHEMA: &str = "OTERYN_NATIVE_SELECTED_LEAF/v1";

pub(super) fn candidate(
    definition: &serde_json::Map<String, Value>,
    admitted: &dyn Fn(&TypedDefinitionRef) -> bool,
) -> Result<Result<Candidate, LeftOut>, ClaimsError> {
    let Some(plain) = plain(definition) else {
        return Ok(Err(LeftOut::UnencodedField));
    };
    super::candidate(&plain, admitted)
}

fn amount(value: &Value) -> Option<(&str, u64)> {
    if !only(value, &["count", "item"])
        || !only(&value["item"], &["family", "key", "revision"])
        || value["item"]["family"] != "Item"
        || value["item"]["revision"] != "definition-r1"
    {
        return None;
    }
    let key = value["item"]["key"].as_str()?;
    let id = key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    let count = value["count"].as_u64()?;
    (id > 0 && (1..=100).contains(&count)).then_some((key, count))
}

fn plain(definition: &serde_json::Map<String, Value>) -> Option<serde_json::Map<String, Value>> {
    let v = Value::Object(definition.clone());
    let s = v.get("native_selection")?;
    let digest = |v: &Value| {
        v.as_str().is_some_and(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        })
    };
    let bounded = |v: &Value, max| {
        v.as_str()
            .is_some_and(|s| !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control))
    };
    if !only(
        &v,
        &[
            "identity",
            "claim",
            "quest",
            "placements",
            "readiness",
            "provenance",
            "definition_profile",
            "native_selection",
        ],
    ) || v["definition_profile"] != PROFILE
        || !only(
            s,
            &[
                "schema",
                "rule",
                "generator",
                "quest",
                "source_claim",
                "source_revision",
                "unique_id",
                "position",
                "original_reward",
                "original_reward_sha256",
                "reward",
                "wiki",
            ],
        )
        || s["schema"] != SCHEMA
        || s["generator"] != "tools/content-schema/reward-claim-authoring/selected_native_leaf.py"
        || !bounded(&s["rule"], 1024)
        || !bounded(&s["quest"], 128)
        || !s["quest"].as_str()?.starts_with("oteryn:quest.")
        || s["source_claim"] != v["provenance"]["pilot_key"]
        || s["source_revision"] != v["provenance"]["pilot_revision"]
        || !bounded(&s["source_claim"], 256)
        || !bounded(&s["source_revision"], 256)
        || !digest(&s["original_reward_sha256"])
    {
        return None;
    }
    let (old_key, old_count) = amount(&s["original_reward"])?;
    let normalized = serde_json::json!({"count":old_count,"item":{"family":"Item","key":old_key,"revision":"definition-r1"}});
    let hash = Sha256::digest(serde_json::to_vec(&normalized).ok()?);
    let hash = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();
    if s["original_reward_sha256"] != hash {
        return None;
    }
    let (selected, _) = amount(&s["reward"])?;
    let wiki = &s["wiki"];
    if !only(
        wiki,
        &[
            "path",
            "title",
            "pageid",
            "revid",
            "sha256",
            "item_id",
            "snapshot_sha256",
        ],
    ) || !bounded(&wiki["path"], 256)
        || !bounded(&wiki["title"], 256)
        || !wiki["path"].as_str().is_some_and(|p| {
            p.starts_with("imports/tibiawiki/")
                && p.ends_with(".json")
                && !p.contains("..")
                && !p.contains('\\')
        })
        || !digest(&wiki["sha256"])
        || !digest(&wiki["snapshot_sha256"])
        || !wiki["pageid"].as_u64().is_some_and(|n| n > 0)
        || !wiki["revid"].as_u64().is_some_and(|n| n > 0)
        || selected != format!("oteryn:item.tibia.i{}", wiki["item_id"].as_u64()?)
    {
        return None;
    }
    let placements = v["placements"].as_array()?;
    let [p] = placements.as_slice() else {
        return None;
    };
    if !only(&p["reward"], &["items"])
        || p["reward"]["items"] != serde_json::json!([s["reward"].clone()])
        || !s["unique_id"]
            .as_u64()
            .is_some_and(|n| (1..=65535).contains(&n))
        || s["position"] != p["source_binding"]["project_position"]
    {
        return None;
    }
    let crystal: Vec<_> = p["source_binding"]["legacy_unique_ids"]
        .as_array()?
        .iter()
        .filter(|id| id["server"] == "crystalserver")
        .collect();
    if crystal.len() != 1 || crystal[0]["unique_id"] != s["unique_id"] {
        return None;
    }
    let mut plain = definition.clone();
    plain.remove("definition_profile");
    plain.remove("native_selection");
    Some(plain)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Result<Value, String> {
        let doc: Value = serde_json::from_str(include_str!(
            "../../../../../../content/interactions/reward_claims/reward-claims-00000-00099.json"
        ))
        .map_err(|e| e.to_string())?;
        doc["records"]
            .as_array()
            .ok_or("records")?
            .iter()
            .find(|r| r["definition"]["definition_profile"] == PROFILE)
            .map(|r| r["definition"].clone())
            .ok_or_else(|| "selected leaf".into())
    }

    #[test]
    fn selected_leaf_delegates_item_admission_after_exact_witness_validation() -> Result<(), String>
    {
        let doc = source()?;
        let d = doc.as_object().ok_or("definition")?;
        let selected = candidate(d, &|_| true)
            .map_err(|e| format!("{e:?}"))?
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(selected.placements.len(), 1);
        assert_eq!(
            selected.placements[0].item.key().as_str(),
            "oteryn:item.tibia.i3050"
        );
        assert!(matches!(
            candidate(d, &|_| false).map_err(|e| format!("{e:?}"))?,
            Err(LeftOut::ItemNotAdmitted)
        ));
        Ok(())
    }

    #[test]
    fn selected_leaf_rejects_mismatched_binding_identity_and_unencoded_fields() -> Result<(), String>
    {
        let original = source()?;
        for edit in 0..7 {
            let mut v = original.clone();
            match edit {
                0 => v["native_selection"]["unique_id"] = 6152.into(),
                1 => v["native_selection"]["wiki"]["item_id"] = 3087.into(),
                2 => v["native_selection"]["position"]["x"] = 1.into(),
                3 => v["native_selection"]["original_reward"]["count"] = 2.into(),
                4 => v["native_selection"]["wiki"]["sha256"] = "unverified".into(),
                5 => v["native_contents"] = serde_json::json!({}),
                _ => v["native_selection"]["unknown"] = true.into(),
            }
            assert!(
                plain(v.as_object().ok_or("definition")?).is_none(),
                "edit {edit}"
            );
        }
        Ok(())
    }
}
