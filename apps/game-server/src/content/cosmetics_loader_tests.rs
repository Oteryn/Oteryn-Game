//! Synthetic authored cosmetics below are raw test data, NOT population, source-truth,
//! account unlock, live sex or current-world admission evidence. Historical Outfit observations
//! are the real byte-exact existing reader output; no DAT reader or native types are replicated.
use super::*;
use serde_json::{Value, json};
use std::path::PathBuf;

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
fn repository() -> PathBuf {
    std::env::var_os("OTERYN_TEST_COSMETICS_REPOSITORY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

struct Fixture {
    manifest: Vec<u8>,
    shards: Vec<(String, Vec<u8>)>,
    document: Value,
}
impl Fixture {
    fn actual_source_raw_authored_document() -> TestResult<Self> {
        let dir = repository().join("imports/cipsoft-appearances/outfits");
        let manifest = std::fs::read(dir.join("manifest.json"))?;
        assert_eq!(
            hex(sha(&manifest)),
            "097cd666b8878f604bf30314080790c4b03866e3a99f4e80c0ca019807b24958"
        );
        let parsed: Value = serde_json::from_slice(&manifest)?;
        let mut shards = Vec::new();
        let mut refs = BTreeMap::new();
        for file in parsed["files"].as_array().ok_or("files")? {
            let name = file["path"].as_str().ok_or("path")?.to_owned();
            let bytes = std::fs::read(dir.join(&name))?;
            let data: Value = serde_json::from_slice(&bytes)?;
            for record in data["records"].as_array().ok_or("records")? {
                refs.insert(record["source_id"].as_u64().ok_or("id")?, json!({"id":record["source_id"],"source_record_sha256":record["source_record_sha256"]}));
            }
            shards.push((name, bytes));
        }
        assert_eq!(refs.len(), 1480);
        let source = json!({"reference":"RAW SYNTHETIC AUTHORING TEST ONLY","revision":"synthetic-r1","sha256":hex(sha(b"raw test citation, not proof of production facts"))});
        let document = json!({
            "schema":"OTERYN_CHARACTER_COSMETICS_CANDIDATE/v1", "content_revision":"candidate-test-r1",
            "source_manifest_sha256":hex(sha(&manifest)),
            "source_appearances_sha256":"2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2",
            "palette":{"key":"oteryn:palette.proposed","revision":"proposal-r1","colours":vec![[0_u8,0,0];133],"source":source},
            "outfits":[{"key":"oteryn:outfit.proposed","revision":"proposal-r1","female":refs.get(&136).ok_or("look136")?,"male":refs.get(&128).ok_or("look128")?,
                "addon_count":2,"starter_female":true,"starter_male":true,"starter_addons_female":0,"starter_addons_male":1,
                "premium":true,"unlock_sources":[],"source":source}],
            "mounts":[{"key":"oteryn:mount.proposed","revision":"proposal-r1","look":refs.get(&139).ok_or("look139")?,"speed_bonus":10,"premium":true,"source":source}],
            "fallbacks":{"female":{"outfit_key":"oteryn:outfit.proposed","colours":[0,1,2,132]},"male":{"outfit_key":"oteryn:outfit.proposed","colours":[132,2,1,0]}}
        });
        Ok(Self {
            manifest,
            shards,
            document,
        })
    }
    fn binding<'a>(&self, bytes: &[u8]) -> ProposedCosmeticsBinding<'a> {
        ProposedCosmeticsBinding {
            document_sha256: sha(bytes),
            content_revision: "candidate-test-r1",
            manifest_sha256: sha(&self.manifest),
            appearances_sha256: hash(
                "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2",
            )
            .unwrap_or([0; 32]),
            client_version: "15.30",
        }
    }
    fn shard_refs(&self) -> Vec<(&str, &[u8])> {
        self.shards
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect()
    }
    fn compile(&self) -> Result<CosmeticsCandidate> {
        let bytes = serde_json::to_vec(&self.document)
            .map_err(|_| CosmeticsDiagnostic::MalformedDocument)?;
        compile_candidate(
            &bytes,
            self.binding(&bytes),
            &self.manifest,
            &self.shard_refs(),
        )
    }
    /// These mutations are deliberately RAW proposed data, not an alternative admitted source.
    fn replace_source(&mut self, shard: usize, mutate: impl FnOnce(&mut Value)) -> TestResult {
        let mut parsed: Value = serde_json::from_slice(&self.shards[shard].1)?;
        mutate(&mut parsed);
        self.shards[shard].1 = serde_json::to_vec(&parsed)?;
        let mut manifest: Value = serde_json::from_slice(&self.manifest)?;
        manifest["files"][shard]["sha256"] = json!(hex(sha(&self.shards[shard].1)));
        self.manifest = serde_json::to_vec(&manifest)?;
        self.document["source_manifest_sha256"] = json!(hex(sha(&self.manifest)));
        Ok(())
    }
}

#[test]
fn actual_staged_source_compiles_raw_candidate_with_sex_looks_palette_and_fallbacks() -> TestResult
{
    let f = Fixture::actual_source_raw_authored_document()?;
    let c = f.compile().map_err(debug)?;
    assert_eq!(c.content_revision(), "candidate-test-r1");
    assert_ne!(c.document_sha256(), [0; 32]);
    assert_eq!(c.look_type("oteryn:outfit.proposed", Sex::Male), Some(128));
    assert_eq!(
        c.look_type("oteryn:outfit.proposed", Sex::Female),
        Some(136)
    );
    assert_eq!(c.look_type("missing", Sex::Female), None);
    assert_eq!(c.palette().len(), 133);
    assert_eq!(
        c.mount_metadata("oteryn:mount.proposed"),
        Some((139, 10, true))
    );
    for sex in [Sex::Female, Sex::Male] {
        let fallback = c.fallback(sex);
        assert_eq!(fallback.addons, 0);
        assert_eq!(fallback.mount_key, None);
        validate_selection(&fallback, sex, &c, &NoEarnedUnlocks).map_err(debug)?;
    }
    Ok(())
}

#[test]
fn native_selection_validator_is_reused_without_unlock_or_premium_authority() -> TestResult {
    let f = Fixture::actual_source_raw_authored_document()?;
    let c = f.compile().map_err(debug)?;
    let mut selection = c.fallback(Sex::Male);
    selection.addons = 1;
    validate_selection(&selection, Sex::Male, &c, &NoEarnedUnlocks).map_err(debug)?;
    assert_eq!(
        validate_selection(&selection, Sex::Female, &c, &NoEarnedUnlocks),
        Err(crate::domain::appearance::AppearanceRejection::AddonNotAuthorized { addon: 1 })
    );
    selection.addons = 0;
    selection.mount_key = Some("oteryn:mount.proposed".into());
    assert_eq!(
        validate_selection(&selection, Sex::Male, &c, &NoEarnedUnlocks),
        Err(crate::domain::appearance::AppearanceRejection::MountNotAuthorized)
    );
    // A candidate view never mints an earned unlock or usable Store entitlement.
    assert!(!c.addon_is_starter(&"oteryn:outfit.proposed".into(), Sex::Male, 0));
    assert!(!c.addon_is_starter(&"oteryn:outfit.proposed".into(), Sex::Male, 9));
    Ok(())
}

#[test]
fn independent_document_digest_revision_and_source_binding_reject_substitution() -> TestResult {
    let f = Fixture::actual_source_raw_authored_document()?;
    let bytes = serde_json::to_vec(&f.document)?;
    let original = f.binding(&bytes);
    let mut altered = bytes.clone();
    altered.push(b' ');
    assert!(matches!(
        compile_candidate(&altered, original, &f.manifest, &f.shard_refs()),
        Err(CosmeticsDiagnostic::DocumentBinding)
    ));
    for change in 0..4 {
        let mut binding = original;
        match change {
            0 => binding.document_sha256 = [0; 32],
            1 => binding.content_revision = "other-r1",
            2 => binding.manifest_sha256 = [1; 32],
            _ => binding.appearances_sha256 = [2; 32],
        }
        assert!(matches!(
            compile_candidate(&bytes, binding, &f.manifest, &f.shard_refs()),
            Err(CosmeticsDiagnostic::DocumentBinding)
        ));
    }
    Ok(())
}

#[test]
fn exact_manifest_and_shard_bytes_are_required_not_self_reported_source_hashes() -> TestResult {
    let mut f = Fixture::actual_source_raw_authored_document()?;
    let bytes = serde_json::to_vec(&f.document)?;
    let original = f.binding(&bytes);
    let mut manifest = f.manifest.clone();
    manifest.push(b' ');
    assert!(matches!(
        compile_candidate(&bytes, original, &manifest, &f.shard_refs()),
        Err(CosmeticsDiagnostic::SourceBinding)
    ));
    f.shards[0].1.push(b' ');
    assert!(matches!(
        f.compile(),
        Err(CosmeticsDiagnostic::ShardDigest { shard: 0 })
    ));
    Ok(())
}

#[test]
fn missing_duplicate_or_relabelled_shards_fail_closed() -> TestResult {
    let mut f = Fixture::actual_source_raw_authored_document()?;
    let last = f.shards.pop().ok_or("last shard")?;
    assert!(matches!(
        f.compile(),
        Err(CosmeticsDiagnostic::MissingOrDuplicateShard)
    ));
    f.shards.push(last);
    f.shards[1].0 = f.shards[0].0.clone();
    assert!(matches!(
        f.compile(),
        Err(CosmeticsDiagnostic::MissingOrDuplicateShard)
    ));
    Ok(())
}

#[test]
fn wrong_source_category_version_namespace_or_dat_identity_is_rejected() -> TestResult {
    for change in 0..5 {
        let mut f = Fixture::actual_source_raw_authored_document()?;
        let mut m: Value = serde_json::from_slice(&f.manifest)?;
        match change {
            0 => m["family"] = json!("Item"),
            1 => m["top_level_field"] = json!(1),
            2 => m["client_version"] = json!("15.32"),
            3 => m["identity_namespace"] = json!("object"),
            _ => m["inputs"]["appearances"]["sha256"] = json!(hex([7; 32])),
        }
        f.manifest = serde_json::to_vec(&m)?;
        f.document["source_manifest_sha256"] = json!(hex(sha(&f.manifest)));
        assert!(matches!(
            f.compile(),
            Err(CosmeticsDiagnostic::SourceBinding)
        ));
    }
    Ok(())
}

#[test]
fn staged_duplicates_bad_order_and_missing_record_provenance_are_rejected() -> TestResult {
    for change in 0..3 {
        let mut f = Fixture::actual_source_raw_authored_document()?;
        f.replace_source(0, |s| match change {
            0 => s["records"][1]["source_id"] = s["records"][0]["source_id"].clone(),
            1 => s["records"][0]["source_index"] = json!(1),
            _ => s["records"][0]["source_record_sha256"] = json!(hex([0; 32])),
        })?;
        assert!(matches!(
            f.compile(),
            Err(CosmeticsDiagnostic::SourceRecord { .. })
        ));
    }
    Ok(())
}

#[test]
fn numeric_look_membership_without_matching_record_provenance_is_insufficient() -> TestResult {
    let mut f = Fixture::actual_source_raw_authored_document()?;
    f.document["outfits"][0]["female"]["source_record_sha256"] = json!(hex([9; 32]));
    assert!(matches!(
        f.compile(),
        Err(CosmeticsDiagnostic::Look {
            family: "Outfit",
            index: 0
        })
    ));
    let mut f = Fixture::actual_source_raw_authored_document()?;
    f.document["outfits"][0]["female"]["id"] = json!(65_535);
    assert!(matches!(f.compile(), Err(CosmeticsDiagnostic::Look { .. })));
    let mut f = Fixture::actual_source_raw_authored_document()?;
    f.document["mounts"][0]["look"]["id"] = json!(0);
    assert!(matches!(
        f.compile(),
        Err(CosmeticsDiagnostic::Look {
            family: "Mount",
            index: 0
        })
    ));
    Ok(())
}

#[test]
fn duplicate_or_cross_family_definition_keys_are_rejected() -> TestResult {
    let mut f = Fixture::actual_source_raw_authored_document()?;
    let outfit = f.document["outfits"][0].clone();
    f.document["outfits"]
        .as_array_mut()
        .ok_or("outfits")?
        .push(outfit);
    assert!(matches!(
        f.compile(),
        Err(CosmeticsDiagnostic::DuplicateKey {
            family: "Outfit",
            index: 1
        })
    ));
    let mut f = Fixture::actual_source_raw_authored_document()?;
    f.document["mounts"][0]["key"] = f.document["outfits"][0]["key"].clone();
    assert!(matches!(
        f.compile(),
        Err(CosmeticsDiagnostic::DuplicateKey {
            family: "Mount",
            index: 0
        })
    ));
    Ok(())
}

#[test]
fn incomplete_unknown_and_duplicate_authored_members_never_default() -> TestResult {
    for field in [
        "female",
        "starter_female",
        "starter_addons_male",
        "premium",
        "unlock_sources",
        "source",
    ] {
        let mut f = Fixture::actual_source_raw_authored_document()?;
        f.document["outfits"][0]
            .as_object_mut()
            .ok_or("outfit")?
            .remove(field);
        assert!(matches!(
            f.compile(),
            Err(CosmeticsDiagnostic::MalformedDocument)
        ));
    }
    let mut f = Fixture::actual_source_raw_authored_document()?;
    f.document["unexpected"] = json!(true);
    assert!(matches!(
        f.compile(),
        Err(CosmeticsDiagnostic::MalformedDocument)
    ));
    let f = Fixture::actual_source_raw_authored_document()?;
    let raw = serde_json::to_string(&f.document)?;
    let raw = raw.replacen(
        "\"content_revision\":",
        "\"content_revision\":\"candidate-test-r1\",\"content_revision\":",
        1,
    );
    assert!(matches!(
        compile_candidate(
            raw.as_bytes(),
            f.binding(raw.as_bytes()),
            &f.manifest,
            &f.shard_refs()
        ),
        Err(CosmeticsDiagnostic::MalformedDocument)
    ));
    Ok(())
}

#[test]
fn native_identity_and_provenance_syntax_are_checked() -> TestResult {
    for change in 0..3 {
        let mut f = Fixture::actual_source_raw_authored_document()?;
        match change {
            0 => f.document["outfits"][0]["key"] = json!(""),
            1 => f.document["outfits"][0]["revision"] = json!("x".repeat(129)),
            _ => f.document["outfits"][0]["source"]["sha256"] = json!(hex([0; 32])),
        }
        assert!(matches!(
            f.compile(),
            Err(CosmeticsDiagnostic::Definition {
                family: "Outfit",
                index: 0
            })
        ));
    }
    Ok(())
}

#[test]
fn eight_addons_are_supported_without_shift_overflow_and_excess_bits_refuse() -> TestResult {
    let mut f = Fixture::actual_source_raw_authored_document()?;
    f.document["outfits"][0]["addon_count"] = json!(8);
    f.document["outfits"][0]["starter_addons_male"] = json!(255);
    let c = f.compile().map_err(debug)?;
    let mut selection = c.fallback(Sex::Male);
    selection.addons = 255;
    validate_selection(&selection, Sex::Male, &c, &NoEarnedUnlocks).map_err(debug)?;
    f.document["outfits"][0]["addon_count"] = json!(9);
    assert!(matches!(
        f.compile(),
        Err(CosmeticsDiagnostic::Definition { .. })
    ));
    f.document["outfits"][0]["addon_count"] = json!(0);
    assert!(matches!(
        f.compile(),
        Err(CosmeticsDiagnostic::Definition { .. })
    ));
    Ok(())
}

#[test]
fn fallback_must_be_per_sex_starter_known_and_in_palette_without_addons_or_mount() -> TestResult {
    for change in 0..3 {
        let mut f = Fixture::actual_source_raw_authored_document()?;
        match change {
            0 => f.document["outfits"][0]["starter_female"] = json!(false),
            1 => f.document["fallbacks"]["female"]["outfit_key"] = json!("oteryn:outfit.missing"),
            _ => f.document["fallbacks"]["female"]["colours"] = json!([133, 0, 0, 0]),
        }
        assert!(matches!(
            f.compile(),
            Err(CosmeticsDiagnostic::Fallback { sex: Sex::Female })
        ));
    }
    for field in ["addons", "mount_key"] {
        let mut f = Fixture::actual_source_raw_authored_document()?;
        f.document["fallbacks"]["male"][field] = json!(1);
        assert!(matches!(
            f.compile(),
            Err(CosmeticsDiagnostic::MalformedDocument)
        ));
    }
    let mut f = Fixture::actual_source_raw_authored_document()?;
    f.document["fallbacks"]
        .as_object_mut()
        .ok_or("fallbacks")?
        .remove("female");
    assert!(matches!(
        f.compile(),
        Err(CosmeticsDiagnostic::MalformedDocument)
    ));
    Ok(())
}

#[test]
fn palette_and_input_allocation_bounds_fail_closed() -> TestResult {
    let mut f = Fixture::actual_source_raw_authored_document()?;
    f.document["palette"]["colours"] = json!([]);
    assert!(matches!(f.compile(), Err(CosmeticsDiagnostic::Palette)));
    f.document["palette"]["colours"] = json!(vec![[0_u8; 3]; 65_536]);
    assert!(matches!(f.compile(), Err(CosmeticsDiagnostic::Palette)));
    let bytes = vec![b' '; JSON_BYTES_MAX + 1];
    assert!(matches!(
        compile_candidate(&bytes, f.binding(&bytes), &f.manifest, &f.shard_refs()),
        Err(CosmeticsDiagnostic::InputTooLarge)
    ));
    Ok(())
}

#[test]
fn existing_unpopulated_outfit_index_and_empty_mount_records_are_not_cosmetics() -> TestResult {
    let f = Fixture::actual_source_raw_authored_document()?;
    for path in [
        "content/cosmetics/outfits/index.json",
        "content/cosmetics/mounts/mounts-00000-00251.json",
    ] {
        let bytes = std::fs::read(repository().join(path))?;
        let parsed: Value = serde_json::from_slice(&bytes)?;
        if path.ends_with("index.json") {
            assert_eq!(parsed["population_state"], "READY_UNPOPULATED");
        } else {
            let records = parsed["records"].as_array().ok_or("mounts")?;
            assert_eq!(records.len(), 252);
            assert!(
                records
                    .iter()
                    .all(|r| r["declaration"]["fields"] == json!([]))
            );
        }
        assert!(matches!(
            compile_candidate(&bytes, f.binding(&bytes), &f.manifest, &f.shard_refs()),
            Err(CosmeticsDiagnostic::MalformedDocument)
        ));
    }
    Ok(())
}
