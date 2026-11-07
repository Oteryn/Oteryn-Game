//! SCENARIO_MATRIX C20c replay coverage.
//!
//! The native client consumes the server command-result payload in
//! `crates/client-ui/src/features/0382-death-protection/protocol.rs::decode`, which delegates to
//! `crates/protocol-oteryn/src/death_protection.rs::DeathProtectionReply::decode`; the visible HUD
//! value is produced by `crates/client-ui/src/features/0382-death-protection/view.rs::notice`.

use super::{DeathProtection, DeathProtectionReply, decode, notice};

#[test]
fn replays_real_death_protection_receipt_and_rejects_invalid_payload_unchanged()
-> Result<(), Box<dyn std::error::Error>> {
    let (payload, expected) = if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = std::path::Path::new(&directory);
        (
            std::fs::read(directory.join("c20c-death-protection.bin"))?,
            std::fs::read_to_string(directory.join("c20c-expected.txt"))?,
        )
    } else {
        let expected = "oteryn:blessing.twist_of_fate".to_owned();
        let payload = DeathProtectionReply(Some(DeathProtection {
            occurrence: [20; 16],
            consumed: vec![expected.clone()],
        }))
        .encode()
        .ok_or("inline C20c fixture did not encode")?;
        (payload, expected)
    };

    let mut state = DeathProtectionReply::default();
    if let Some(decoded) = decode(&payload) {
        state = decoded;
    }
    let receipt = state.0.as_ref().ok_or("C20c receipt was absent")?;
    assert_eq!(receipt.consumed.join("\n"), expected);
    assert_eq!(notice(receipt), "Death — consumed: Twist Of Fate");

    let before = state.clone();
    let mut invalid = payload;
    invalid.push(0);
    if let Some(decoded) = decode(&invalid) {
        state = decoded;
    }
    assert_eq!(state, before);
    Ok(())
}
