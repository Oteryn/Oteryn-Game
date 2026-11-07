//! Decode the donor official-appearance Rotate flag independently from movement assets.
use super::tests::with_flags;
use super::*;

#[test]
fn official_rotate_flag_is_decoded_from_field_22() -> Result<(), prost::DecodeError> {
    // Canary pinned src/protobuf/appearances.proto:154: optional bool rotate = 22.
    for (payload, expected) in [
        (&[][..], false),
        (&[0xb0, 0x01, 0][..], false),
        (&[0xb0, 0x01, 1][..], true),
    ] {
        let flags = FlagsMsg::decode(payload)?;
        assert_eq!(
            with_flags(1, flags).map(|appearance| appearance.rotatable),
            Some(expected)
        );
    }
    Ok(())
}
