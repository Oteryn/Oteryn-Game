use super::*;

fn reply(disposition: u64, code: StaffCode) -> StaffToolsReply {
    StaffToolsReply {
        result: StaffResult {
            code,
            disposition,
            control_seq: 2,
            link_sha256: vec![1; 32],
            mute_until_ms: 0,
        },
        info: String::new(),
    }
}
#[test]
fn staff_tools_reply_preserves_signed_operation_result_and_info() -> Result<(), StaffToolsWireError>
{
    let mut value = reply(1, StaffCode::Ok);
    value.info = "Character: Example\nPosition: 100,200,7".into();
    assert_eq!(decode_reply(&encode_reply(&value)?)?, value);
    value.info = "x".repeat(MAX_INFO_BYTES);
    let bytes = encode_reply(&value)?;
    assert!(bytes.len() <= MAX_REPLY_BYTES);
    assert_eq!(decode_reply(&bytes)?, value);
    value.info.push('x');
    assert!(encode_reply(&value).is_err());
    Ok(())
}
#[test]
fn staff_tools_reply_refuses_info_on_refusal_or_receipt() -> Result<(), StaffToolsWireError> {
    for mut value in [reply(2, StaffCode::RoleRevoked), reply(3, StaffCode::Ok)] {
        assert_eq!(decode_reply(&encode_reply(&value)?)?, value);
        value.info = "private inspection".into();
        assert!(encode_reply(&value).is_err());
        let mut bytes = Vec::new();
        push_message_field(&mut bytes, 1, &staff::encode_result(&value.result)?);
        push_message_field(&mut bytes, 2, value.info.as_bytes());
        assert!(decode_reply(&bytes).is_err());
    }
    Ok(())
}
#[test]
fn staff_tools_reply_rejects_unknown_duplicate_truncated_and_utf8()
-> Result<(), StaffToolsWireError> {
    let value = reply(1, StaffCode::Ok);
    let bytes = encode_reply(&value)?;
    for end in 0..bytes.len() {
        assert!(decode_reply(&bytes[..end]).is_err());
    }
    let mut duplicate = bytes.clone();
    duplicate.extend_from_slice(&bytes);
    assert!(decode_reply(&duplicate).is_err());
    let mut unknown = bytes.clone();
    unknown.extend([24, 1]);
    assert!(decode_reply(&unknown).is_err());
    let mut invalid_utf8 = bytes.clone();
    invalid_utf8.extend([18, 1, 255]);
    assert!(decode_reply(&invalid_utf8).is_err());
    let mut duplicate_info = bytes.clone();
    duplicate_info.extend([18, 0, 18, 0]);
    assert!(decode_reply(&duplicate_info).is_err());
    let mut over = bytes;
    push_message_field(&mut over, 2, &vec![b'x'; MAX_INFO_BYTES + 1]);
    assert!(decode_reply(&over).is_err());
    assert!(decode_reply(&vec![0; MAX_REPLY_BYTES + 1]).is_err());
    Ok(())
}
