#![allow(clippy::expect_used)]
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

#[test]
fn client_codec_round_trips_server_codec() {
    let request = Request {
        operation: Operation::Edit,
        number: 7,
        text: "friend".into(),
        icon: 3,
        notify: true,
        groups: vec![2, 4],
    };
    let (kind, bytes) = encode_request(&request).expect("valid request");
    assert_eq!(decode_request(kind, &bytes), Ok(request));
    let bytes = encode_result(8, 9, 7, 2, "{\"entries\":[]}").expect("valid result");
    assert_eq!(
        decode_result(&bytes),
        Ok(ResultView {
            code: 8,
            revision: 9,
            number: 7,
            wait: 2,
            view: "{\"entries\":[]}".into()
        })
    );
}
