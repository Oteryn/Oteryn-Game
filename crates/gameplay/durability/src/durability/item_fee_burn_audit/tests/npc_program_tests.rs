use super::*;

#[test]
fn npc_program_tag18_is_retained_as_a_distinct_known_cause() {
    // An empty tag18 message must survive decoding so its malformed proof is
    // rejected by the closed fee validator instead of silently losing its cause.
    let decoded = OneItemFeeBurnCauseV1::decode([0x92, 0x01, 0x00].as_slice()).unwrap();
    assert!(decoded.cause.is_some());
    assert_eq!(decoded.encode_to_vec(), [0x92, 0x01, 0x00]);
}
