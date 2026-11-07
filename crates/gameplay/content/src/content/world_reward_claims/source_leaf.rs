//! Closed native source format for one literal written reward and its once flag.
use super::*;

pub(super) const PROFILE: &str = "native_source_leaf_v1";
const SCHEMA: &str = "OTERYN_NATIVE_SOURCE_LEAF/v1";

pub(super) fn candidate(
    definition: &serde_json::Map<String, Value>,
    admitted: &dyn Fn(&TypedDefinitionRef) -> bool,
) -> Result<Result<Candidate, LeftOut>, ClaimsError> {
    let Some(candidate) = parse(definition) else {
        return Ok(Err(LeftOut::UnencodedField));
    };
    if candidate.placements.iter().any(|p| !admitted(&p.item)) {
        return Ok(Err(LeftOut::ItemNotAdmitted));
    }
    Ok(Ok(candidate))
}

fn parse(definition: &serde_json::Map<String, Value>) -> Option<Candidate> {
    let doc = Value::Object(definition.clone());
    let digest = |v: &Value, length| {
        v.as_str().is_some_and(|s| {
            s.len() == length
                && s.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        })
    };
    if !only(
        &doc,
        &[
            "identity",
            "definition_profile",
            "claim",
            "quest",
            "readiness",
            "provenance",
            "placements",
            "native_source",
            "native_contents",
        ],
    ) || doc["definition_profile"] != PROFILE
        || doc["readiness"] != "ready"
        || doc["claim"] != serde_json::json!({"per":"character","repeat":{"kind":"once"}})
        || !only(&doc["identity"], &["key", "revision"])
        || doc["identity"]["revision"] != "reward-claim-r1"
        || !doc["identity"]["key"]
            .as_str()?
            .starts_with("oteryn:reward-claim.")
        || !only(&doc["quest"], &["family", "key", "revision"])
        || doc["quest"]["family"] != "Quest"
        || doc["quest"]["revision"] != "quest-r1"
        || !doc["quest"]["key"].as_str()?.starts_with("oteryn:quest.")
        || !only(&doc["provenance"], &["generator", "rule"])
        || doc["provenance"]["generator"]
            != "tools/content-schema/reward-claim-authoring/native_source_leaf.py"
        || !doc["provenance"]["rule"]
            .as_str()
            .is_some_and(|s| !s.is_empty() && s.len() <= 1024)
    {
        return None;
    }
    let source = &doc["native_source"];
    if !only(
        source,
        &[
            "schema",
            "repository",
            "revision",
            "path",
            "table",
            "unique_id",
            "line",
            "sha256",
            "git_blob_sha1",
            "completion",
        ],
    ) || source["schema"] != SCHEMA
        || !matches!(
            source["repository"].as_str()?,
            "zimbadev/crystalserver" | "opentibiabr/canary"
        )
        || !digest(&source["revision"], 40)
        || !digest(&source["sha256"], 64)
        || !digest(&source["git_blob_sha1"], 40)
        || !source["line"]
            .as_u64()
            .is_some_and(|n| (1..=u64::from(u32::MAX)).contains(&n))
        || !source["path"].as_str().is_some_and(|s| {
            s.len() <= 256
                && s.ends_with(".lua")
                && !s.contains("..")
                && !s.contains(['\\', '\0'])
                && (s.starts_with("data-global/") || s.starts_with("data-otservbr-global/"))
        })
        || !source["table"].as_str().is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 64
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
    {
        return None;
    }
    let uid = source["unique_id"].as_u64()?;
    let completion = source["completion"].as_str()?;
    if !(1..=65535).contains(&uid)
        || completion.len() > 512
        || !completion.starts_with("Storage.")
        || !completion
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_'))
    {
        return None;
    }
    let projection = &doc["native_contents"];
    if !only(
        projection,
        &["schema", "revision", "source_sha256", "placements"],
    ) || projection["schema"] != SCHEMA
        || projection["revision"] != source["revision"]
        || projection["source_sha256"] != source["sha256"]
    {
        return None;
    }
    let placements = doc["placements"].as_array()?;
    let recipes = projection["placements"].as_array()?;
    if placements.is_empty() || placements.len() > 16 || placements.len() != recipes.len() {
        return None;
    }
    let mut parsed = Vec::new();
    for (placement, recipe) in placements.iter().zip(recipes) {
        if !only(
            recipe,
            &[
                "unique_id",
                "line",
                "root",
                "capacity",
                "contents",
                "root_text",
                "completion",
            ],
        ) || recipe["unique_id"].as_u64() != Some(uid)
            || recipe["line"] != source["line"]
            || !recipe.get("capacity")?.is_null()
            || recipe["contents"] != serde_json::json!([])
            || recipe["completion"] != serde_json::json!({"kind":"storage","key":completion})
            || placement["reward"]["items"]
                != serde_json::json!([{"item":recipe["root"],"count":1}])
            || !only(&placement["reward"], &["items", "written_text"])
        {
            return None;
        }
        let body = recipe["root_text"].as_str()?;
        if body.is_empty()
            || body.len() > 7996
            || body.chars().count() > 4000
            || body.contains('\0')
        {
            return None;
        }
        let hash: String = Sha256::digest(body.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if placement["reward"]["written_text"]
            != serde_json::json!({"item":recipe["root"],
            "text_ref":{"length":body.chars().count(),"placeholders":[],"sha256":hash}})
        {
            return None;
        }
        let mut ordinary = placement.clone();
        ordinary["reward"] = serde_json::json!({"items":placement["reward"]["items"]});
        let mut candidate = candidate_placement(&ordinary, false).ok()??;
        if candidate.crystal_unique_id != Some(uid) || candidate.quest_transition.is_none() {
            return None;
        }
        candidate.root_text = Some(body.to_owned());
        parsed.push(candidate);
    }
    Some(Candidate {
        revision: "reward-claim-r1".into(),
        placements: parsed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Result<Value, ClaimsError> {
        let shard: Value = serde_json::from_str(include_str!(
            "../../../../../../content/interactions/reward_claims/reward-claim-source-leaves.json"
        ))
        .map_err(|_| ClaimsError("native source fixture"))?;
        Ok(shard["records"][0]["definition"].clone())
    }

    #[test]
    fn literal_source_book_requires_the_complete_exact_projection() -> Result<(), ClaimsError> {
        let source = source()?;
        assert!(parse(source.as_object().ok_or(ClaimsError("definition"))?).is_some());
        for case in 0..9 {
            let mut value = source.clone();
            match case {
                0 => value["unknown"] = true.into(),
                1 => value["native_contents"]["placements"][0]["root_text"] = "substituted".into(),
                2 => {
                    value["native_contents"]["placements"][0]["root"]["key"] =
                        "oteryn:item.tibia.i2820".into()
                }
                3 => value["native_source"]["unique_id"] = 20004.into(),
                4 => value["native_source"]["sha256"] = "invalid".into(),
                5 => value["native_contents"]["placements"][0]["root_charges_remaining"] = 1.into(),
                6 => value["placements"][0]["reward"]["items"][0]["count"] = 2.into(),
                7 => {
                    value["native_contents"]["placements"][0]["completion"]["key"] =
                        "Storage.Other".into()
                }
                _ => value["claim"]["repeat"]["kind"] = "daily".into(),
            }
            assert!(
                parse(value.as_object().ok_or(ClaimsError("definition"))?).is_none(),
                "case {case}"
            );
        }
        Ok(())
    }

    #[test]
    fn source_written_reward_still_requires_native_item_admission() -> Result<(), ClaimsError> {
        let value = source()?;
        let def = value.as_object().ok_or(ClaimsError("definition"))?;
        assert!(matches!(
            candidate(def, &|_| false)?,
            Err(LeftOut::ItemNotAdmitted)
        ));
        let result =
            candidate(def, &|_| true)?.map_err(|_| ClaimsError("valid source was refused"))?;
        assert_eq!(result.placements.len(), 1);
        assert_eq!(result.placements[0].crystal_unique_id, Some(20003));
        assert!(
            result.placements[0]
                .root_text
                .as_ref()
                .is_some_and(|s| s.starts_with("The map shows"))
        );
        Ok(())
    }
    #[test]
    fn source_leaf_respects_the_same_excluded_owner_scope() -> Result<(), ClaimsError> {
        let mut value = source()?;
        value["quest"]["key"] = "oteryn:quest.dawnport_quest".into();
        assert!(matches!(
            super::super::candidate(value.as_object().ok_or(ClaimsError("definition"))?, &|_| {
                true
            })?,
            Err(LeftOut::OwnerExcluded)
        ));
        Ok(())
    }
}
