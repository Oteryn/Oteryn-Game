//! Exercise authored grants through the existing native cosmetics admission path.
use super::*;
use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, UnlockFact};

fn packet_with_grants() -> Value {
    let mut packet = cosmetics_test_packet();
    let key = packet["authoring"]["document"]["outfits"][0]["key"].clone();
    let mappings = serde_json::json!([{
        "grant_key": "quest:wardrobe:reward",
        "target": {"kind":"Addon", "outfit_key":key, "addon":2},
        "compatible_provenance":["historical-r7", "historical-r9"]
    }]);
    packet["authoring"]["document"]["grant_mappings"] = mappings.clone();
    let mut promotion = packet["authoring"]["promotions"][0].clone();
    promotion["path"] = "/grant_mappings".into();
    promotion["value"] = mappings;
    packet["authoring"]["promotions"]
        .as_array_mut()
        .unwrap()
        .push(promotion);
    packet
}

#[test]
fn appearance_grant_mappings_authorize_only_authored_identity_and_provenance() {
    let packet = packet_with_grants();
    let view = native_cosmetics::decode(&serde_json::to_vec(&packet).unwrap())
        .unwrap()
        .unwrap();
    let key = view.outfit_keys().next().unwrap();
    for revision in ["historical-r7", "historical-r9"] {
        assert_eq!(
            view.earned_unlock(
                "quest:wardrobe:reward",
                revision,
                EarnedCosmetic::Addon(key, 2)
            ),
            UnlockFact::Compatible
        );
    }
    assert_eq!(
        view.earned_unlock(
            "quest:wardrobe:reward",
            view.content_revision(),
            EarnedCosmetic::Addon(key, 2)
        ),
        UnlockFact::Incompatible
    );
    for target in [
        EarnedCosmetic::Outfit(key),
        EarnedCosmetic::Addon(key, 1),
        EarnedCosmetic::Mount(key),
    ] {
        assert_eq!(
            view.earned_unlock("quest:wardrobe:reward", "historical-r7", target),
            UnlockFact::Absent
        );
    }
    assert_eq!(
        view.earned_unlock("unknown", "historical-r7", EarnedCosmetic::Addon(key, 2)),
        UnlockFact::Absent
    );
}

#[test]
fn appearance_grant_mappings_require_promotion_and_valid_unique_targets() {
    let original = packet_with_grants();
    let mut unpromoted = original.clone();
    unpromoted["authoring"]["promotions"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(native_cosmetics::decode(&serde_json::to_vec(&unpromoted).unwrap()).is_err());
    for invalid in [
        serde_json::json!({"target":{"kind":"Addon","outfit_key":"oteryn:missing","addon":2}}),
        serde_json::json!({"target":{"kind":"Addon","outfit_key":original["authoring"]["document"]["outfits"][0]["key"],"addon":0}}),
        serde_json::json!({"compatible_provenance":[]}),
        serde_json::json!({"compatible_provenance":["same", "same"]}),
    ] {
        let mut packet = original.clone();
        let mapping = packet["authoring"]["document"]["grant_mappings"][0]
            .as_object_mut()
            .unwrap();
        mapping.extend(invalid.as_object().unwrap().clone());
        let value = packet["authoring"]["document"]["grant_mappings"].clone();
        packet["authoring"]["promotions"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap()["value"] = value;
        assert!(native_cosmetics::decode(&serde_json::to_vec(&packet).unwrap()).is_err());
    }
}

#[test]
fn marriage_grant_resolves_through_sex_qualified_authorization() {
    use crate::domain::appearance::Sex;
    let mut packet = cosmetics_test_packet();
    let shards = packet["source_shards"].as_array().unwrap().clone();
    for (sex, id) in [("female", 329), ("male", 328)] {
        let record = shards
            .iter()
            .flat_map(|shard| {
                serde_json::from_str::<Value>(shard["json"].as_str().unwrap()).unwrap()["records"]
                    .as_array()
                    .unwrap()
                    .clone()
            })
            .find(|record| record["source_id"] == id)
            .unwrap();
        packet["authoring"]["document"]["outfits"][0][sex] = serde_json::json!({
            "id":id,"source_record_sha256":record["source_record_sha256"]});
    }
    let outfit = packet["authoring"]["document"]["outfits"][0].clone();
    for promotion in packet["authoring"]["promotions"].as_array_mut().unwrap() {
        if promotion["path"] == "/outfits/0" {
            promotion["value"] = outfit.clone();
        }
        for sex in ["female", "male"] {
            if promotion["path"] == format!("/outfits/0/{sex}") {
                promotion["value"] = outfit[sex].clone();
            }
        }
    }
    let view = native_cosmetics::decode(&serde_json::to_vec(&packet).unwrap())
        .unwrap()
        .unwrap();
    let key = view.outfit_keys().next().unwrap();
    for sex in [Sex::Female, Sex::Male] {
        assert_eq!(
            view.earned_unlock_for_sex(
                MARRIAGE_GRANT_KEY,
                MARRIAGE_PROVENANCE,
                EarnedCosmetic::Outfit(key),
                sex
            ),
            UnlockFact::Compatible
        );
        assert_eq!(
            view.earned_unlock_for_sex(
                MARRIAGE_GRANT_KEY,
                "wrong-provenance",
                EarnedCosmetic::Outfit(key),
                sex
            ),
            UnlockFact::Incompatible
        );
        assert_eq!(
            view.earned_unlock_for_sex(
                MARRIAGE_GRANT_KEY,
                MARRIAGE_PROVENANCE,
                EarnedCosmetic::Addon(key, 1),
                sex
            ),
            UnlockFact::Incompatible
        );
    }
}
