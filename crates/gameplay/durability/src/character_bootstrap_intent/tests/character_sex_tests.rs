#![allow(clippy::expect_used)]
use super::*;

#[test]
fn character_sex_is_required_and_bound_by_authenticated_creation() {
    let male =
        decode_producer_response(&wire(|_| {}), OPERATION_ID).expect("valid appearance fixture");
    let female = decode_producer_response(
        &wire(|v| v["requested_sex"] = "Female".into()),
        OPERATION_ID,
    )
    .expect("valid appearance fixture");
    assert_eq!(male.requested_sex(), Sex::Male);
    assert_eq!(female.requested_sex(), Sex::Female);
    assert_ne!(male.binding(), female.binding());
    assert_eq!(male.binding().last(), Some(&2));
    assert_eq!(female.binding().last(), Some(&1));
    for invalid in [
        wire(|v| {
            v.as_object_mut()
                .expect("valid appearance fixture")
                .remove("requested_sex");
        }),
        wire(|v| v["requested_sex"] = serde_json::Value::Null),
        wire(|v| v["requested_sex"] = "Unknown".into()),
        wire(|v| v["requested_sex"] = 2.into()),
        wire(|v| v["contract_version"] = 2.into()),
    ] {
        assert!(decode_producer_response(&invalid, OPERATION_ID).is_err());
    }
}
