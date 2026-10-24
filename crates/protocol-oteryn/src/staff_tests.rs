#![allow(clippy::expect_used, reason = "wire fixtures")]
use super::*;
#[test]
fn staff_wire_roundtrip_and_exact_limits() {
    let e = SignedEnvelope {
        principal: "a".repeat(MAX_PRINCIPAL_BYTES),
        canonical: vec![b'x'; MAX_SIGNED_BYTES],
        signature: vec![b's'; MAX_SIGNATURE_BYTES],
    };
    let bytes = encode_envelope(&e).expect("bounded envelope");
    assert!(bytes.len() <= MAX_ENVELOPE_BYTES);
    assert_eq!(decode_envelope(&bytes), Ok(e.clone()));
    let mut over = e.clone();
    over.canonical.push(b'x');
    assert!(encode_envelope(&over).is_err());
    over = e.clone();
    over.signature.push(b'x');
    assert!(encode_envelope(&over).is_err());
    over = e;
    over.principal.push('x');
    assert!(encode_envelope(&over).is_err());
    let r = StaffResult {
        code: StaffCode::Ok,
        disposition: 1,
        control_seq: u64::MAX,
        link_sha256: vec![255; 32],
        mute_until_ms: u64::MAX,
    };
    assert_eq!(decode_result(&encode_result(&r).expect("result")), Ok(r));
}
#[test]
fn staff_wire_rejects_missing_duplicate_unknown_truncated_and_unsigned() {
    let e = SignedEnvelope {
        principal: "operator".into(),
        canonical: b"{}".to_vec(),
        signature: b"sig".to_vec(),
    };
    let bytes = encode_envelope(&e).expect("envelope");
    for cut in 0..bytes.len() {
        assert!(decode_envelope(&bytes[..cut]).is_err());
    }
    let mut duplicate = bytes.clone();
    duplicate.extend_from_slice(&bytes);
    assert!(decode_envelope(&duplicate).is_err());
    let mut unknown = bytes.clone();
    unknown.extend_from_slice(&[32, 1]);
    assert!(decode_envelope(&unknown).is_err());
    let mut bad = e.clone();
    bad.signature.clear();
    assert!(encode_envelope(&bad).is_err());
    let mut bad = e;
    bad.principal = "*".into();
    assert!(encode_envelope(&bad).is_err());
    assert!(decode_envelope(&vec![0; MAX_ENVELOPE_BYTES + 1]).is_err());
    assert!(decode_result(&[8, 1, 16, 1]).is_err());
}
