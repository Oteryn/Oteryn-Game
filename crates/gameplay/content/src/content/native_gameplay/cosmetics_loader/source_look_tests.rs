//! Current-Tibia Mage/Summoner addon tasks award cross-family sex/look pairs.
use super::*;
#[test]
fn srv7_cross_family_source_addon_looks_are_admitted_without_unscoped_entitlement() -> TestResult {
    use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, UnlockFact};
    let mut fixture = Fixture::actual_source_raw_authored_document()?;
    let mut looks = BTreeMap::new();
    for (_, bytes) in &fixture.shards {
        let shard: Value = serde_json::from_slice(bytes)?;
        for row in shard["records"].as_array().ok_or("look records")? {
            if let Some(id) = row["source_id"].as_u64()
                && [130, 138, 133, 141].contains(&id)
            {
                looks.insert(
                    id,
                    json!({"id":id,"source_record_sha256":row["source_record_sha256"]}),
                );
            }
        }
    }
    let mage = "oteryn:outfit.proposed_mage";
    let summoner = "oteryn:outfit.proposed_summoner";
    let base = fixture.document["outfits"][0].clone();
    let mut mage_row = base.clone();
    mage_row["key"] = json!(mage);
    mage_row["male"] = looks.get(&130).ok_or("source MageMale130")?.clone();
    mage_row["female"] = looks.get(&138).ok_or("source MageFemale138")?.clone();
    let mut summoner_row = base;
    summoner_row["key"] = json!(summoner);
    summoner_row["male"] = looks.get(&133).ok_or("source SummonerMale133")?.clone();
    summoner_row["female"] = looks.get(&141).ok_or("source SummonerFemale141")?.clone();
    fixture.document["outfits"] = json!([mage_row, summoner_row]);
    fixture.document["fallbacks"]["male"]["outfit_key"] = json!(mage);
    fixture.document["fallbacks"]["female"]["outfit_key"] = json!(mage);
    fixture.document["grant_mappings"] = json!([
        {"grant_key":"oteryn:test.source.mage-hat-male","target":{"kind":"LookAddon","outfit_key":mage,"sex":"male","look_type":130,"addon":2},"compatible_provenance":["source-test-v1"]},
        {"grant_key":"oteryn:test.source.summoner-hat-female","target":{"kind":"LookAddon","outfit_key":summoner,"sex":"female","look_type":141,"addon":2},"compatible_provenance":["source-test-v1"]}
    ]);
    let content = fixture.compile().map_err(debug)?;
    for (grant, key) in [
        ("oteryn:test.source.mage-hat-male", mage),
        ("oteryn:test.source.summoner-hat-female", summoner),
    ] {
        assert_eq!(
            content.earned_unlock(grant, "source-test-v1", EarnedCosmetic::Addon(key, 2)),
            UnlockFact::Absent,
            "unscoped legacy callers cannot turn one exact source look into both canonical sex looks"
        );
    }
    use crate::domain::appearance::{AccountUnlocks, EarnedCosmetics, Sex};
    let facts = vec![
        (
            "oteryn:test.source.mage-hat-male".to_owned(),
            "source-test-v1".to_owned(),
        ),
        (
            "oteryn:test.source.summoner-hat-female".to_owned(),
            "source-test-v1".to_owned(),
        ),
    ];
    for (sex, expected_mage, expected_summoner) in
        [(Sex::Male, true, false), (Sex::Female, false, true)]
    {
        let scoped = EarnedCosmetics {
            content: &content,
            facts: &facts,
        }
        .with_sex(sex);
        assert_eq!(
            scoped.addon(&mage.to_owned(), 2) == UnlockFact::Compatible,
            expected_mage
        );
        assert_eq!(
            scoped.addon(&summoner.to_owned(), 2) == UnlockFact::Compatible,
            expected_summoner
        );
    }
    let bytes = serde_json::to_vec(&facts)?;
    let restored: Vec<(String, String)> = serde_json::from_slice(&bytes)?;
    let reloaded = fixture.compile().map_err(debug)?;
    assert_eq!(
        EarnedCosmetics {
            content: &reloaded,
            facts: &restored
        }
        .with_sex(Sex::Female)
        .addon(&mage.to_owned(), 2),
        UnlockFact::Absent,
        "reloaded source facts cannot unlock MageFemale138 from the Male130 grant"
    );
    assert_eq!(
        EarnedCosmetics {
            content: &reloaded,
            facts: &restored
        }
        .with_sex(Sex::Male)
        .addon(&summoner.to_owned(), 2),
        UnlockFact::Absent,
        "reloaded source facts cannot unlock SummonerMale133 from the Female141 grant"
    );
    fixture.document["grant_mappings"][0]["target"]["sex"] = json!("female");
    assert!(
        fixture.compile().is_err(),
        "a numeric look claimed under the wrong canonical sex must fail admission"
    );
    Ok(())
}

#[test]
fn srv7_npc_program_source_citation_is_exact_and_never_an_unlock() -> TestResult {
    let mut fixture = Fixture::actual_source_raw_authored_document()?;
    let expected = sha(b"raw qualification-test source program bytes");
    let citation = json!({"reference":"content/cosmetics/npc-acquisitions.json","revision":"source-program-v1","sha256":hex(expected)});
    fixture.document["outfits"][0]["unlock_sources"] = json!([citation.clone()]);
    let content = fixture.compile().map_err(debug)?;
    assert_eq!(content.npc_acquisition_program_digest(), Some(expected));
    assert_ne!(
        content.npc_acquisition_program_digest(),
        Some(sha(b"substituted source program bytes")),
        "the generic owner must compare the actual embedded bytes to the admitted citation"
    );
    use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, UnlockFact};
    assert_eq!(
        content.earned_unlock(
            "content/cosmetics/npc-acquisitions.json",
            "source-program-v1",
            EarnedCosmetic::Outfit("oteryn:outfit.proposed")
        ),
        UnlockFact::Absent,
        "a source citation is never an account entitlement"
    );
    let mut drift = citation;
    drift["sha256"] = json!(hex(sha(b"other program")));
    let original_citation = fixture.document["outfits"][0]["unlock_sources"][0].clone();
    fixture.document["outfits"][0]["unlock_sources"] = json!([original_citation, drift]);
    assert_eq!(
        fixture
            .compile()
            .map_err(debug)?
            .npc_acquisition_program_digest(),
        None,
        "conflicting programme source citations fail closed"
    );
    fixture.document["outfits"][0]["unlock_sources"][0]["sha256"] = json!("00");
    assert!(
        fixture.compile().is_err(),
        "malformed source digests do not enter an admitted document"
    );
    Ok(())
}
