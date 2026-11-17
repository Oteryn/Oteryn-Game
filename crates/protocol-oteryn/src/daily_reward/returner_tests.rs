use super::returner::*;
#[test]
fn returner_codec_requires_server_entitlement_identity() -> Result<(), &'static str> {
    let id = [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];
    for state in 0..=2 {
        let reply = Reply {
            state,
            id: if state == 0 { None } else { Some(id) },
        };
        assert_eq!(Reply::decode(&reply.encode()?)?, reply);
    }
    assert!(Reply { state: 1, id: None }.encode().is_err());
    assert!(decode_claim(&[0; 16]).is_err());
    assert_eq!(decode_claim(&id)?, id);
    Ok(())
}
