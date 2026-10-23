use super::*;
#[test]
fn vip_codec_bounds_and_strict_decoding() {
    assert_eq!(
        decode_request(50, &[8, 12]).map(|r| r.operation),
        Ok(Operation::Query)
    );
    assert!(decode_request(48, &[8, 12]).is_err());
    assert!(decode_request(50, &[8, 12, 8, 12]).is_err());
    assert!(decode_request(50, &[8, 12, 0]).is_err());
    assert!(decode_request(48, &[8, 1, 26, 1, 255]).is_err());
    assert!(decode_request(48, &vec![0; MAX_REQUEST_BYTES + 1]).is_err());
    let mut add = vec![8, 1, 26, 5];
    add.extend(b"Alice");
    assert_eq!(decode_request(48, &add).map(|r| r.text), Ok("Alice".into()));
    for n in 0..add.len() {
        assert!(decode_request(48, &add[..n]).is_err());
    }
    assert!(encode_result(8, 1, 0, 0, "").is_err());
    assert!(encode_result(1, 1, 0, 0, &"x".repeat(MAX_RESULT_BYTES)).is_err());
    assert_eq!(encode_result(1, 7, 2, 0, "{}").map(|v| v.len()), Ok(12));
}
