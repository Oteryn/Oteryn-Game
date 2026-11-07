#![allow(clippy::expect_used)]
use crate::appearance::{Change, ChangeResult, Context, Selection};

fn change() -> Change {
    Change {
        context: Context {
            content_generation: [1; 32],
            cosmetics_digest: [2; 32],
        },
        expected_root_revision: 1,
        prior: None,
        selection: Selection {
            outfit_key: "oteryn:outfit.citizen".into(),
            colours: [0, 1, 2, 132],
            addons: 0,
            mount_key: None,
        },
        content_revision: "r1".into(),
    }
}
#[test]
fn appearance_change_roundtrip_requires_explicit_zero_scalars() {
    let change = change();
    let bytes = change.encode().expect("valid appearance fixture");
    assert_eq!(
        Change::decode(&bytes).expect("valid appearance fixture"),
        change
    );
    let selection = change.selection.encode().expect("valid appearance fixture");
    assert_eq!(&selection[selection.len() - 2..], &[24, 0]);
    assert!(Selection::decode(&selection[..selection.len() - 2]).is_err());
    for invalid in [
        vec![8, 0],
        vec![8, 1],
        vec![8, 2, 16, 1],
        vec![8, 3],
        vec![8, 1, 16, 0],
        vec![8, 2, 8, 2],
        vec![8, 2, 24, 0],
    ] {
        assert!(ChangeResult::decode(&invalid).is_err());
    }
    assert_eq!(
        ChangeResult::Rejected
            .encode()
            .expect("valid appearance fixture"),
        vec![8, 2]
    );
    for result in [ChangeResult::Committed(u64::MAX), ChangeResult::Rejected] {
        assert_eq!(
            ChangeResult::decode(&result.encode().expect("valid appearance fixture"))
                .expect("valid appearance fixture"),
            result
        );
    }
}
#[test]
fn appearance_change_rejects_missing_unknown_duplicate_and_overflow_fields() {
    let bytes = change().encode().expect("valid appearance fixture");
    for field in 0..bytes.len() {
        assert!(Change::decode(&bytes[..field]).is_err());
    }
    for suffix in [
        &[8, 0][..],
        &[50, 0],
        &[56, 0],
        &[48, 0],
        &[24, 255, 255, 255, 255, 255, 255, 255, 255, 255, 2],
    ] {
        let mut invalid = bytes.clone();
        invalid.extend_from_slice(suffix);
        assert!(Change::decode(&invalid).is_err());
    }
    let mut invalid = change();
    invalid.selection.outfit_key = "x".repeat(513);
    assert!(invalid.encode().is_err());
    invalid = change();
    invalid.content_revision = "x".repeat(129);
    assert!(invalid.encode().is_err());
    invalid = change();
    invalid.selection.outfit_key = "bad\u{0000}".into();
    assert!(invalid.encode().is_err());
}

#[path = "appearance_tests/mappings_tests.rs"]
mod mappings_tests;
