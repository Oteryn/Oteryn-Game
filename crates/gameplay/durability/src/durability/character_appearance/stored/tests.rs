use super::*;
use crate::domain::appearance::{
    AccountUnlocks, AppearanceContent, AppearanceRejection, Sex, UnlockFact, displayed_look,
    validate_selection,
};

// Test-only catalogue: keys, sex and compatibility never come from a production request.
struct Content;
impl AppearanceContent<String> for Content {
    fn palette_size(&self) -> u16 {
        133
    }
    fn outfit_addon_count(&self, key: &String) -> Option<u8> {
        matches!(key.as_str(), "starter" | "earned" | "store").then_some(2)
    }
    fn outfit_is_starter(&self, key: &String, sex: Sex) -> bool {
        key == "starter" && sex == Sex::Male
    }
    fn addon_is_starter(&self, _: &String, _: Sex, _: u8) -> bool {
        false
    }
    fn mount_exists(&self, key: &String) -> bool {
        key == "horse"
    }
}
struct NoUnlocks;
impl AccountUnlocks<String> for NoUnlocks {
    fn outfit(&self, _: &String) -> UnlockFact {
        UnlockFact::Absent
    }
    fn addon(&self, _: &String, _: u8) -> UnlockFact {
        UnlockFact::Absent
    }
    fn mount(&self, _: &String) -> UnlockFact {
        UnlockFact::Absent
    }
}

fn stored() -> DurableAppearanceSelection {
    decode_selection_fields(
        "starter".into(),
        &[0, 1, 2, 132],
        0,
        None,
        "content-1".into(),
        "2",
    )
    .expect("valid fixture")
}

#[test]
fn preserves_unknown_keys_and_provenance_without_reinterpreting() {
    let selection = decode_selection_fields(
        "gone".into(),
        &[0, 1, 2, 132],
        0,
        Some("gone-mount".into()),
        "old-world".into(),
        "9",
    )
    .expect("historical selection need not exist in active content");
    assert_eq!(selection.selection.outfit_key, "gone");
    assert_eq!(selection.selection.mount_key.as_deref(), Some("gone-mount"));
    assert_eq!(selection.content_revision, "old-world");
    let fallback = stored();
    assert_eq!(
        displayed_look(
            &selection.selection,
            &fallback.selection,
            Sex::Male,
            &Content,
            &NoUnlocks
        )
        .outfit_key,
        "starter"
    );
    assert_eq!(selection.content_revision, "old-world");
    assert_eq!(selection.selection.outfit_key, "gone");
}

#[test]
fn decoding_rejects_lossy_numbers_invalid_revisions_and_byte_overflow() {
    for (colours, addons, revision, provenance) in [
        (vec![0, 0, 0], 0, "2", "ok".to_owned()),
        (vec![-1, 0, 0, 0], 0, "2", "ok".to_owned()),
        (vec![65536, 0, 0, 0], 0, "2", "ok".to_owned()),
        (vec![0, 0, 0, 0], -1, "2", "ok".to_owned()),
        (vec![0, 0, 0, 0], 256, "2", "ok".to_owned()),
        (vec![0, 0, 0, 0], 0, "0", "ok".to_owned()),
        (vec![0, 0, 0, 0], 0, "18446744073709551616", "ok".to_owned()),
        (vec![0, 0, 0, 0], 0, "2", String::new()),
        (vec![0, 0, 0, 0], 0, "2", "é".repeat(65)),
    ] {
        assert!(matches!(
            decode_selection_fields(
                "starter".into(),
                &colours,
                addons,
                None,
                provenance,
                revision
            ),
            Err(AppearanceReadError::InvalidStoredState)
        ));
    }
    assert!(
        decode_selection_fields(
            "old".into(),
            &[65535; 4],
            255,
            None,
            "é".repeat(64),
            "18446744073709551615"
        )
        .is_ok()
    );
}

#[test]
fn native_validator_rejects_each_locked_component_and_palette_boundary() {
    let mut value = stored().selection;
    assert_eq!(
        validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
        Ok(())
    );
    value.outfit_key = "earned".into();
    assert_eq!(
        validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
        Err(AppearanceRejection::OutfitNotAuthorized)
    );
    value.outfit_key = "store".into();
    assert_eq!(
        validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
        Err(AppearanceRejection::OutfitNotAuthorized)
    );
    value.outfit_key = "starter".into();
    value.addons = 1;
    assert_eq!(
        validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
        Err(AppearanceRejection::AddonNotAuthorized { addon: 1 })
    );
    value.addons = 4;
    assert_eq!(
        validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
        Err(AppearanceRejection::AddonOutOfRange { addon: 3 })
    );
    value.addons = 0;
    value.mount_key = Some("horse".into());
    assert_eq!(
        validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
        Err(AppearanceRejection::MountNotAuthorized)
    );
    value.mount_key = None;
    value.colours[3] = 133;
    assert_eq!(
        validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
        Err(AppearanceRejection::ColourOutOfPalette { slot: 3 })
    );
}
