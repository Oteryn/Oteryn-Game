//! Command 348 with capability 485: the exact upgrade tier after the four definition bytes.
use oteryn_client_runtime::cui3::hotkey_equip::{Request, TieredRequest};
use std::num::NonZeroU32;

/// The command 348 payload. With 485 selected (`tiered`) it carries `upgrade_tier` (0 = an
/// unforged copy, at most 10, otherwise `None`); without it the four-byte request is unchanged.
#[must_use]
pub fn equip_payload(definition: NonZeroU32, tiered: bool, upgrade_tier: u8) -> Option<Vec<u8>> {
    TieredRequest {
        request: Request { definition },
        upgrade_tier: tiered.then_some(upgrade_tier),
    }
    .encode()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
