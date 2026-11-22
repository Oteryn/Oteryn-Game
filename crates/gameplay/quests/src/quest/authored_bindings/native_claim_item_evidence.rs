//! A complete native container projection can resolve a historical SOURCE Item
//! hold for nonreward progress. It never authorizes completion or delivery.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

const SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../tools/content-schema/reward-claim-authoring/container_native_source.json"
));

struct Proof {
    owner: Value,
    key: String,
    revision: String,
}

fn proof(definition: &Value, digest: &str, revision: &Value) -> Option<Proof> {
    let native = &definition["native_contents"];
    let source = &definition["source_variant"]["source_claim"];
    if definition["definition_profile"] != "authored_variant_v1"
        || !definition["data_holds"].as_array()?.is_empty()
        || native["schema"] != "OTERYN_NATIVE_CONTAINER_REWARD/v1"
        || native["source_sha256"] != digest
        || &native["revision"] != revision
        || definition["quest"]["family"] != "Quest"
        || definition["quest"] != source["quest"]
        || source["identity"]["key"] != definition["provenance"]["pilot_key"]
        || source["identity"]["revision"] != definition["provenance"]["pilot_revision"]
    {
        return None;
    }
    let placements = definition["placements"].as_array()?;
    let projections = native["placements"].as_array()?;
    if placements.is_empty() || placements.len() != projections.len() {
        return None;
    }
    for (placement, projection) in placements.iter().zip(projections) {
        let reward = &placement["reward"];
        let container = &reward["container"];
        let expected = reward["items"].as_array()?;
        let contents = projection["contents"].as_array()?;
        if container["family"] != "Item"
            || projection["root"] != *container
            || !reward
                .as_object()?
                .keys()
                .all(|key| matches!(key.as_str(), "items" | "container"))
            || contents.len() != expected.len()
            || !contents.iter().zip(expected).all(|(native, original)| {
                native["item"] == original["item"] && native["count"] == original["count"]
            })
            || contents.len() > 20
            || !contents.iter().all(|child| {
                child["item"]["family"] == "Item"
                    && child["count"]
                        .as_u64()
                        .is_some_and(|count| (1..=100).contains(&count))
            })
            || !placement["source_binding"]["legacy_unique_ids"]
                .as_array()?
                .iter()
                .any(|binding| {
                    binding["server"] == "crystalserver"
                        && binding["unique_id"].as_u64().is_some()
                        && binding["unique_id"] == projection["unique_id"]
                })
        {
            return None;
        }
    }
    Some(Proof {
        owner: definition["quest"].clone(),
        key: source["identity"]["key"].as_str()?.into(),
        revision: source["identity"]["revision"].as_str()?.into(),
    })
}

pub(super) fn resolves(quest: &Value, hold: &Value) -> bool {
    if hold["code"] != "claim_item_semantics_missing" {
        return false;
    }
    static PROOFS: OnceLock<Vec<Proof>> = OnceLock::new();
    let proofs = PROOFS.get_or_init(|| {
        let digest: String = Sha256::digest(SOURCE.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let Ok(source) = serde_json::from_str::<Value>(SOURCE) else {
            return Vec::new();
        };
        let mut proofs = Vec::new();
        for (_, document) in super::NATIVE_CLAIM_SHARDS {
            let Ok(shard) = serde_json::from_str::<Value>(document) else {
                return Vec::new();
            };
            let Some(records) = shard["records"].as_array() else {
                return Vec::new();
            };
            if records.len() > 4096 {
                return Vec::new();
            }
            proofs.extend(
                records
                    .iter()
                    .filter_map(|row| proof(&row["definition"], &digest, &source["revision"])),
            );
        }
        proofs
    });
    proofs.iter().any(|proof| {
        proof.owner == quest["source_refs"]["quest"]
            && hold["source_key"] == proof.key
            && quest["source_refs"]["claims"]
                .as_array()
                .is_some_and(|claims| {
                    claims.iter().any(|claim| {
                        claim["family"] == "RewardClaim"
                            && claim["key"] == proof.key
                            && claim["revision"] == proof.revision
                    })
                })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_native_container_evidence_is_exact_and_never_partial() -> Result<(), String> {
        let mut found = None;
        for (_, document) in super::super::NATIVE_CLAIM_SHARDS {
            let shard: Value = serde_json::from_str(document).map_err(|error| error.to_string())?;
            for row in shard["records"].as_array().ok_or("claim records")? {
                if row["definition"]["identity"]["key"] == "oteryn:reward-claim.kv.draconia1" {
                    found = Some(row["definition"].clone());
                }
            }
        }
        let definition = found.ok_or("Draconia native container")?;
        let digest: String = Sha256::digest(SOURCE.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let source: Value = serde_json::from_str(SOURCE).map_err(|error| error.to_string())?;
        assert!(proof(&definition, &digest, &source["revision"]).is_some());
        for mutation in 0..7 {
            let mut invalid = definition.clone();
            match mutation {
                0 => invalid["data_holds"] = serde_json::json!([{"category":"item"}]),
                1 => invalid["quest"]["key"] = "canary:quest/other".into(),
                2 => invalid["native_contents"]["source_sha256"] = "0".repeat(64).into(),
                3 => {
                    invalid["native_contents"]["placements"][0]["contents"] = serde_json::json!([])
                }
                4 => invalid["native_contents"]["placements"][0]["unique_id"] = 0.into(),
                5 => invalid["native_contents"]["placements"][0]["contents"][0]["count"] = 2.into(),
                _ => invalid["provenance"]["pilot_key"] = "canary:reward-claim/other".into(),
            }
            assert!(
                proof(&invalid, &digest, &source["revision"]).is_none(),
                "accepted mutation {mutation}"
            );
        }
        Ok(())
    }
}

/// Dreamer's Challenge uses these existing reward owners for side chests.
/// The source definition keeps its historical holds for the source inventory.
pub(super) fn resolves_dreamer_auxiliary(quest: &Value, hold: &Value) -> bool {
    if quest["identity"]["key"] != "oteryn:quest.dreamer_s_challenge_quest" {
        return false;
    }
    let expected = match (hold["code"].as_str(), hold["source_key"].as_str()) {
        (
            Some("claim_native_lowering_missing"),
            Some("canary:reward-claim/quest/u7_9/dreamers_challenge/bplong_sword"),
        ) => "bplong_sword",
        (
            Some("claim_native_lowering_missing"),
            Some("canary:reward-claim/quest/u7_9/dreamers_challenge/chest_book"),
        ) => "chest_book",
        (
            Some("claim_native_lowering_missing" | "claim_item_semantics_missing"),
            Some("canary:reward-claim/quest/u7_9/dreamers_challenge/chests_wine"),
        ) => "chests_wine",
        _ => return false,
    };
    static ADMITTED: OnceLock<Vec<Proof>> = OnceLock::new();
    ADMITTED
        .get_or_init(dreamer_auxiliary_proofs)
        .iter()
        .any(|proof| {
            proof.owner == quest["source_refs"]["quest"]
                && hold["source_key"] == proof.key
                && proof.key.ends_with(expected)
                && quest["source_refs"]["claims"]
                    .as_array()
                    .is_some_and(|claims| {
                        claims.iter().any(|claim| {
                            claim["key"] == proof.key && claim["revision"] == proof.revision
                        })
                    })
        })
}

fn dreamer_auxiliary_proofs() -> Vec<Proof> {
    let Ok(source) = serde_json::from_str::<Value>(SOURCE) else {
        return Vec::new();
    };
    let digest = format!("{:x}", Sha256::digest(SOURCE.as_bytes()));
    let mut proofs = Vec::new();
    for (_, document) in super::NATIVE_CLAIM_SHARDS {
        let Ok(shard) = serde_json::from_str::<Value>(document) else {
            return Vec::new();
        };
        let Some(records) = shard["records"].as_array() else {
            return Vec::new();
        };
        proofs.extend(
            records
                .iter()
                .filter_map(|row| dreamer_auxiliary_proof(&row["definition"], &source, &digest)),
        );
    }
    proofs
}

fn dreamer_auxiliary_proof(definition: &Value, source: &Value, digest: &str) -> Option<Proof> {
    let (suffix, uids): (_, &[_]) = match definition["identity"]["key"].as_str()? {
        "oteryn:reward-claim.quest.u7_9.dreamers_challenge.bplong_sword" => {
            ("bplong_sword", &[6195])
        }
        "oteryn:reward-claim.quest.u7_9.dreamers_challenge.chest_book" => ("chest_book", &[6196]),
        "oteryn:reward-claim.quest.u7_9.dreamers_challenge.chests_wine" => {
            ("chests_wine", &[6197, 6198, 6199, 6200])
        }
        _ => return None,
    };
    let source_key = format!("canary:reward-claim/quest/u7_9/dreamers_challenge/{suffix}");
    let native = &definition["native_contents"];
    let source_ref = &definition["source_variant"]["source_claim"];
    let placements = definition["placements"].as_array()?;
    let projected = native["placements"].as_array()?;
    if !definition["data_holds"].as_array()?.is_empty()
        || definition["definition_profile"] != "authored_variant_v1"
        || definition["provenance"]["pilot_key"] != source_key
        || source_ref["identity"]["key"] != source_key
        || source_ref["identity"]["revision"] != definition["provenance"]["pilot_revision"]
        || definition["quest"] != source_ref["quest"]
        || native["schema"] != "OTERYN_NATIVE_CONTAINER_REWARD/v1"
        || native["source_sha256"] != digest
        || native["revision"] != source["revision"]
        || placements.len() != uids.len()
        || projected.len() != uids.len()
    {
        return None;
    }
    for ((placement, projection), &uid) in placements.iter().zip(projected).zip(uids) {
        let evidence = source[if uid == 6196 {
            "leaf_entries"
        } else {
            "entries"
        }]
        .as_array()?
        .iter()
        .find(|entry| entry["unique_id"] == uid)?;
        if !dreamer_placement_matches(placement, projection, evidence, source, uid) {
            return None;
        }
    }
    Some(Proof {
        owner: definition["quest"].clone(),
        key: source_key,
        revision: source_ref["identity"]["revision"].as_str()?.into(),
    })
}

fn dreamer_placement_matches(
    placement: &Value,
    projection: &Value,
    evidence: &Value,
    source: &Value,
    uid: u64,
) -> bool {
    let setting = &evidence["setting"];
    let binding = &placement["source_binding"];
    let Some(rewards) = setting["reward"].as_array() else {
        return false;
    };
    let Some(contents) = projection["contents"].as_array() else {
        return false;
    };
    let item_matches = |item: &Value, number: &Value| {
        item["family"] == "Item"
            && item["revision"] == "definition-r1"
            && number
                .as_u64()
                .is_some_and(|number| item["key"] == format!("oteryn:item.tibia.i{number}"))
    };
    if projection["unique_id"] != uid
        || projection["line"] != evidence["line"]
        || projection["completion"]["kind"] != "storage"
        || projection["completion"]["key"] != setting["storage"]["expr"]
        || binding["project_position"] != setting["itemPos"]
        || placement["appearance_tibia_id"] != setting["itemId"]
        || !binding["legacy_unique_ids"].as_array().is_some_and(|ids| {
            ids.iter()
                .any(|id| id["server"] == "crystalserver" && id["unique_id"] == uid)
        })
    {
        return false;
    }
    let text = || {
        source["text_entries"]
            .as_array()?
            .iter()
            .find(|entry| entry["unique_id"] == uid)
    };
    if uid == 6196 {
        let [reward] = rewards.as_slice() else {
            return false;
        };
        return contents.is_empty()
            && projection["capacity"].is_null()
            && item_matches(&projection["root"], &reward[0])
            && reward[1] == 1
            && text().is_some_and(|entry| projection["root_text"] == entry["text"]);
    }
    item_matches(&projection["root"], &setting["container"])
        && projection["capacity"] == if uid == 6195 { 20 } else { 8 }
        && contents.len() == rewards.len()
        && contents.iter().zip(rewards).all(|(item, reward)| {
            item_matches(&item["item"], &reward[0])
                && item["count"] == reward[1]
                && if reward[0] == 2816 {
                    text().is_some_and(|entry| item["text"] == entry["text"])
                } else if reward[0] == 2874 {
                    item["fluid_subtype"] == 2
                } else {
                    item.get("text").is_none() && item.get("fluid_subtype").is_none()
                }
        })
}
