#![allow(clippy::expect_used)]

use super::*;

const WORLD: &str = "01890a5d-ac96-7ab2-80e7-0123456789ab";
const CHANNEL: &str = "oteryn_chat_01890a5dac967ab280e70123456789ab";

#[test]
fn the_channel_name_is_a_safe_world_scoped_identifier() {
    assert_eq!(channel_name(WORLD).as_deref(), Ok(CHANNEL));
    assert_eq!(CHANNEL.len(), 44);
    for variant in ['8', '9', 'a', 'b'] {
        let world = format!("01890a5d-ac96-7ab2-{variant}0e7-0123456789ab");
        assert_eq!(
            channel_name(&world),
            Ok(format!(
                "oteryn_chat_01890a5dac967ab2{variant}0e70123456789ab"
            )),
            "{world}"
        );
    }
    for bad in [
        "",
        "world_a",
        "01890A5D-AC96-7AB2-80E7-0123456789AB",
        "01890a5dac967ab280e70123456789ab",
        "01890a5d-ac96-7ab2-80e7-0123456789a;",
        "01890a5d-ac96-7ab2-80e7-0123456789abc",
        // Shape-valid but not UUIDv7: version nibble 4, 1 or 0.
        "01890a5d-ac96-4ab2-80e7-0123456789ab",
        "01890a5d-ac96-1ab2-80e7-0123456789ab",
        "01890a5d-ac96-0ab2-80e7-0123456789ab",
        // Version 7 but not the RFC 9562 variant: nibble 0, 7, c or f.
        "01890a5d-ac96-7ab2-00e7-0123456789ab",
        "01890a5d-ac96-7ab2-70e7-0123456789ab",
        "01890a5d-ac96-7ab2-c0e7-0123456789ab",
        "01890a5d-ac96-7ab2-f0e7-0123456789ab",
    ] {
        assert_eq!(channel_name(bad), Err(RelayError::BadChannel), "{bad}");
    }
}
