use super::*;

#[test]
fn raid_warnings_preserve_existing_cyclopedia_wire_bytes() -> Result<(), MapWireError> {
    const LEGACY: [u8; 30] = [
        0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0,
    ];
    assert_eq!(crate::cyclopedia::decode_request(72, &[])?.operation, 0);
    let legacy = crate::cyclopedia::decode_result(&LEGACY)?;
    assert_eq!(crate::cyclopedia::encode_wire_result(&legacy)?, LEGACY);
    assert!(decode(&LEGACY).is_err());

    let warnings = [Warning {
        area: 23,
        subarea: Some(109),
    }];
    let bytes = encode(&warnings)?;
    assert_eq!(decode(&bytes)?, warnings);
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    Ok(())
}
