#![allow(clippy::expect_used)]
use super::*;
#[test]
fn house_move_out_strict_codec() {
    for action in 0..=2 {
        let r = HouseMoveOutRequest {
            house: "oteryn:content.house.test".into(),
            action,
            due_at_unix_ms: if action == 1 { i64::MAX } else { 0 },
        };
        let bytes = encode_house_move_out_request(&r).expect("request");
        assert_eq!(decode_house_move_out_request(&bytes), Ok(r));
        for end in 0..bytes.len() {
            assert!(decode_house_move_out_request(&bytes[..end]).is_err());
        }
        let mut duplicate = bytes.clone();
        duplicate.extend_from_slice(&[16, action]);
        assert!(decode_house_move_out_request(&duplicate).is_err());
        let mut unknown = bytes;
        unknown.extend_from_slice(&[32, 1]);
        assert!(decode_house_move_out_request(&unknown).is_err());
    }
    assert!(decode_house_move_out_request(&[0; MAX_REQUEST_BYTES + 1]).is_err());
    assert!(decode_house_move_out_request(&[10, 1, 255, 16, 0, 24, 0]).is_err());
    for state in 0..=4 {
        let mut id = [1; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let r = HouseMoveOutReply {
            operation: (state != 0).then_some(id),
            state,
            due_at_unix_ms: if state == 0 { 0 } else { 100 },
            census_count: if matches!(state, 2 | 3) { 101 } else { 0 },
            delivered: if state == 3 { 101 } else { 0 },
        };
        let bytes = encode_house_move_out_reply(&r).expect("reply");
        assert_eq!(decode_house_move_out_reply(&bytes), Ok(r));
        for end in 0..bytes.len() {
            assert!(decode_house_move_out_reply(&bytes[..end]).is_err());
        }
    }
    assert!(decode_house_move_out_reply(&[0; MAX_REPLY_BYTES + 1]).is_err());
}
