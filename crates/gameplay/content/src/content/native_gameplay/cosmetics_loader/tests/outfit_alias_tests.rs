//! Aliases are promoted identities, never inferred merely from equal appearance IDs.
use super::*;
use crate::domain::appearance::EarnedCosmetic;

fn fixture() -> TestResult<(Fixture, String, String)> {
    let mut f = Fixture::actual_source_raw_authored_document()?;
    let mut alias = f.document["outfits"][0].clone();
    let canonical = alias["key"].as_str().ok_or("canonical key")?.to_owned();
    let legacy = format!("{canonical}/legacy");
    alias["key"] = legacy.clone().into();
    f.document["outfits"]
        .as_array_mut()
        .ok_or("outfits")?
        .push(alias);
    f.document["grant_mappings"] = json!([
        {"grant_key":"quest:legacy-base", "target":{"kind":"Outfit","outfit_key":legacy},
         "compatible_provenance":["historical-r1"],"outfit_aliases":[canonical]},
        {"grant_key":"quest:canonical-addon", "target":{"kind":"Addon","outfit_key":canonical,"addon":1},
         "compatible_provenance":["historical-r1"],"outfit_aliases":[legacy]}
    ]);
    Ok((f, legacy, canonical))
}

#[test]
fn explicit_outfit_aliases_share_grants_in_both_directions_without_crossing_addon_bits()
-> TestResult {
    let (f, legacy, canonical) = fixture()?;
    let content = f.compile().map_err(debug)?;
    assert!(
        matches!(content.authored_grant_target("quest:legacy-base"), Some(EarnedCosmetic::Outfit(key)) if key == legacy)
    );
    assert_eq!(
        content.look_type(&legacy, Sex::Male),
        content.look_type(&canonical, Sex::Male)
    );
    assert_eq!(
        content.earned_unlock(
            "quest:legacy-base",
            "historical-r1",
            EarnedCosmetic::Outfit(&canonical)
        ),
        UnlockFact::Compatible
    );
    assert_eq!(
        content.earned_unlock(
            "quest:canonical-addon",
            "historical-r1",
            EarnedCosmetic::Addon(&legacy, 1)
        ),
        UnlockFact::Compatible
    );
    assert_eq!(
        content.earned_unlock(
            "quest:canonical-addon",
            "historical-r1",
            EarnedCosmetic::Addon(&legacy, 2)
        ),
        UnlockFact::Absent
    );
    assert_eq!(
        content.earned_unlock(
            "quest:legacy-base",
            "wrong-revision",
            EarnedCosmetic::Outfit(&canonical)
        ),
        UnlockFact::Incompatible
    );
    assert_eq!(
        content.earned_unlock(
            "quest:legacy-base",
            "historical-r1",
            EarnedCosmetic::Addon(&canonical, 1)
        ),
        UnlockFact::Absent
    );
    Ok(())
}

#[test]
fn equal_looks_without_explicit_aliases_do_not_share_owned_grants() -> TestResult {
    let (mut f, _, canonical) = fixture()?;
    for mapping in f.document["grant_mappings"]
        .as_array_mut()
        .ok_or("mappings")?
    {
        mapping
            .as_object_mut()
            .ok_or("mapping")?
            .remove("outfit_aliases");
    }
    let content = f.compile().map_err(debug)?;
    assert_eq!(
        content.earned_unlock(
            "quest:legacy-base",
            "historical-r1",
            EarnedCosmetic::Outfit(&canonical)
        ),
        UnlockFact::Absent
    );
    Ok(())
}

#[test]
fn document_level_outfit_alias_maps_are_refused() -> TestResult {
    let (mut f, legacy, canonical) = fixture()?;
    for mapping in f.document["grant_mappings"]
        .as_array_mut()
        .ok_or("mappings")?
    {
        mapping
            .as_object_mut()
            .ok_or("mapping")?
            .remove("outfit_aliases");
    }
    let body = serde_json::to_string(&f.document)?;
    let alias = serde_json::to_string(&legacy)?;
    let target = serde_json::to_string(&canonical)?;
    let bytes = format!(
        "{{\"outfit_aliases\":{{{alias}:{target},{alias}:{target}}},{}",
        &body[1..]
    )
    .into_bytes();
    assert!(matches!(
        compile_candidate(&bytes, f.binding(&bytes), &f.manifest, &f.shard_refs()),
        Err(CosmeticsDiagnostic::MalformedDocument)
    ));
    Ok(())
}

#[test]
fn outfit_aliases_refuse_unknown_targets_self_aliases_changed_masks_and_source_looks() -> TestResult
{
    for mode in 0..5 {
        let (mut f, legacy, canonical) = fixture()?;
        let last = f.document["outfits"].as_array().ok_or("outfits")?.len() - 1;
        match mode {
            0 => f.document["grant_mappings"][0]["outfit_aliases"] = json!(["outfit:absent"]),
            1 => f.document["grant_mappings"][0]["outfit_aliases"] = json!([legacy]),
            2 => f.document["outfits"][last]["addon_count"] = 0.into(),
            3 => f.document["outfits"][last]["female"]["id"] = 139.into(),
            _ => {
                f.document["grant_mappings"][0]["outfit_aliases"] =
                    json!([canonical.clone(), canonical])
            }
        }
        assert!(f.compile().is_err(), "alias mutation {mode}");
    }
    Ok(())
}

#[test]
fn grant_aliases_preserve_each_authored_outfits_independent_eligibility() -> TestResult {
    let (mut f, legacy, canonical) = fixture()?;
    let last = f.document["outfits"].as_array().ok_or("outfits")?.len() - 1;
    f.document["outfits"][last]["starter_female"] = false.into();
    let content = f.compile().map_err(debug)?;
    assert!(!content.outfit_is_starter(&legacy, Sex::Female));
    assert!(content.outfit_is_starter(&canonical, Sex::Female));
    assert_eq!(
        content.earned_unlock(
            "quest:legacy-base",
            "historical-r1",
            EarnedCosmetic::Outfit(&canonical)
        ),
        UnlockFact::Compatible
    );
    Ok(())
}
