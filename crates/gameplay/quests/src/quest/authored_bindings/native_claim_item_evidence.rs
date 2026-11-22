//! Native claim projections reconcile historical SOURCE holds against their
//! admitted owner. Container Item holds still authorize only nonreward progress.
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

impl Proof {
    fn matches(&self, quest: &Value, hold: &Value) -> bool {
        self.owner == quest["source_refs"]["quest"]
            && hold["source_key"] == self.key
            && quest["source_refs"]["claims"]
                .as_array()
                .is_some_and(|claims| {
                    claims.iter().any(|claim| {
                        claim["family"] == "RewardClaim"
                            && claim["key"] == self.key
                            && claim["revision"] == self.revision
                    })
                })
    }
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
    proofs.iter().any(|proof| proof.matches(quest, hold))
}

/// The archived RewardBook is a standalone native text claim. Preserve the
/// SOURCE core digest and reconcile only its exact generated native lowering.
pub(super) fn resolves_thieves_book(quest: &Value, hold: &Value) -> bool {
    const QUEST: &str = "oteryn:quest.the_thieves_guild_quest";
    const CLAIM: &str = "oteryn:reward-claim.quest.u8_2.the_thieves_guild_quest.reward_book";
    const SOURCE_CLAIM: &str = "canary:reward-claim/quest/u8_2/the_thieves_guild_quest/reward_book";
    const CONTENTS_SHA256: &str =
        "c8f3451cdd6cfc42836542ad5ec06c5650180ab60292b514294d3c5a873db7a0";
    if quest["identity"]["key"] != QUEST
        || hold["code"] != "claim_native_lowering_missing"
        || hold["source_key"] != SOURCE_CLAIM
    {
        return false;
    }

    static PROOF: OnceLock<Option<Proof>> = OnceLock::new();
    let proof = PROOF.get_or_init(|| {
        for (_, document) in super::NATIVE_CLAIM_SHARDS {
            let shard: Value = serde_json::from_str(document).ok()?;
            for row in shard["records"].as_array()? {
                let claim = &row["definition"];
                if claim["identity"]["key"] != CLAIM {
                    continue;
                }
                let native = &claim["native_contents"];
                let bytes = serde_json::to_vec(native).ok()?;
                let digest: String = Sha256::new()
                    .chain_update(bytes)
                    .chain_update(b"\n")
                    .finalize()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                if claim["definition_profile"] != "authored_variant_v1"
                    || !claim["data_holds"].as_array()?.is_empty()
                    || claim["provenance"]["pilot_key"] != SOURCE_CLAIM
                    || digest != CONTENTS_SHA256
                {
                    return None;
                }
                return Some(Proof {
                    owner: claim["quest"].clone(),
                    key: SOURCE_CLAIM.into(),
                    revision: claim["provenance"]["pilot_revision"].as_str()?.into(),
                });
            }
        }
        None
    });
    proof
        .as_ref()
        .is_some_and(|proof| proof.matches(quest, hold))
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
