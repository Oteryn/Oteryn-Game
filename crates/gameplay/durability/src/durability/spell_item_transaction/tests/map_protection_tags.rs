#![allow(clippy::unwrap_used)]
use super::*;
#[test]
fn source_otbm_action_unique_tags_are_not_lost_by_named_codec() {
    assert_eq!(
        source_map_protection_tags(&serde_json::json!({"5":42,"4":3,"15":0})).unwrap(),
        (true, true)
    );
    assert_eq!(
        source_map_protection_tags(&serde_json::json!({"5":0,"4":0})).unwrap(),
        (false, false)
    );
    assert!(source_map_protection_tags(&serde_json::json!({"unique":42})).is_err());
    for value in [
        serde_json::json!(-1),
        serde_json::json!(65536),
        serde_json::json!("42"),
        serde_json::json!(true),
    ] {
        assert!(source_map_protection_tags(&serde_json::json!({"5":value})).is_err());
    }
}
