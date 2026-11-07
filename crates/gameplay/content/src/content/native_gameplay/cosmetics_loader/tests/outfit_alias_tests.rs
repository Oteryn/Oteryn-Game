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
    f.document["outfit_aliases"] = json!({legacy.clone(): canonical.clone()});
    f.document["grant_mappings"] = json!([
        {"grant_key":"quest:legacy-base", "target":{"kind":"Outfit","outfit_key":legacy},
         "compatible_provenance":["historical-r1"]},
        {"grant_key":"quest:canonical-addon", "target":{"kind":"Addon","outfit_key":canonical,"addon":1},
         "compatible_provenance":["historical-r1"]}
    ]);
    Ok((f, legacy, canonical))
}

#[test]
fn explicit_outfit_aliases_share_grants_in_both_directions_without_crossing_addon_bits()
-> TestResult {
    let (f, legacy, canonical) = fixture()?;
    let content = f.compile().map_err(debug)?;
    assert!(content.is_outfit_alias(&legacy));
    assert!(!content.is_outfit_alias(&canonical));
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
    f.document
        .as_object_mut()
        .ok_or("document")?
        .remove("outfit_aliases");
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
fn duplicate_outfit_alias_keys_are_refused_even_when_the_values_agree() -> TestResult {
    let (mut f, legacy, canonical) = fixture()?;
    f.document
        .as_object_mut()
        .ok_or("document")?
        .remove("outfit_aliases");
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
fn outfit_aliases_refuse_unknown_targets_cycles_changed_masks_and_reduced_premium() -> TestResult {
    for mode in 0..5 {
        let (mut f, legacy, canonical) = fixture()?;
        let last = f.document["outfits"].as_array().ok_or("outfits")?.len() - 1;
        match mode {
            0 => f.document["outfit_aliases"][&legacy] = "outfit:absent".into(),
            1 => f.document["outfit_aliases"][&canonical] = legacy.clone().into(),
            2 => f.document["outfits"][last]["addon_count"] = 1.into(),
            3 => {
                f.document["outfits"][last]["premium"] = true.into();
                f.document["outfits"][0]["premium"] = false.into();
            }
            _ => f.document["outfits"][last]["starter_female"] = false.into(),
        }
        assert!(f.compile().is_err(), "alias mutation {mode}");
    }
    Ok(())
}
