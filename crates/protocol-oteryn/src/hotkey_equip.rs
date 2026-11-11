//! PKG5: equip/unequip by the active generation's item definition reference.
use std::num::NonZeroU32;

pub const CAPABILITY_HOTKEY_EQUIP_V1: u32 = 179;
/// Selects nested carried sources and started-copy preference on command 348.
pub const CAPABILITY_HOTKEY_EQUIP_CARRIED_V1: u32 = 180;
pub const COMMAND_TYPE_HOTKEY_EQUIP: u32 = 348;
/// Appends one upgrade tier byte to command 348; the server equips only that forged copy.
pub const CAPABILITY_HOTKEY_EQUIP_UPGRADE_TIER_V1: u32 = 485;
/// Classification upgrade tiers 0..=10; zero is an unforged copy.
pub const MAX_UPGRADE_TIER: u8 = 10;

/// The existing equipment move refusal vocabulary, also used by command 348.
pub fn decode_reply(
    bytes: &[u8],
) -> Result<crate::item_view::ItemMoveOutcome, crate::item_view::ItemViewWireError> {
    crate::item_view::decode_item_move_result_with_equip_drop(bytes, true)
}

/// Four bytes, unsigned big endian; zero and trailing bytes are invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub definition: NonZeroU32,
}

impl Request {
    pub fn encode(self) -> [u8; 4] {
        self.definition.get().to_be_bytes()
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        Some(Self {
            definition: NonZeroU32::new(u32::from_be_bytes(bytes.try_into().ok()?))?,
        })
    }
}

/// Command 348 with capability 485: the four definition bytes, then the exact tier.
/// Without the capability the payload stays the four-byte [`Request`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TieredRequest {
    pub request: Request,
    /// `None` only for a session without capability 485.
    pub upgrade_tier: Option<u8>,
}

impl TieredRequest {
    pub fn encode(self) -> Option<Vec<u8>> {
        let mut out = self.request.encode().to_vec();
        if let Some(tier) = self.upgrade_tier {
            if tier > MAX_UPGRADE_TIER {
                return None;
            }
            out.push(tier);
        }
        Some(out)
    }

    pub fn decode_selected(tiered: bool, bytes: &[u8]) -> Option<Self> {
        if !tiered {
            return Request::decode(bytes).map(|request| Self {
                request,
                upgrade_tier: None,
            });
        }
        let (&tier, definition) = bytes.split_last()?;
        if tier > MAX_UPGRADE_TIER {
            return None;
        }
        Some(Self {
            request: Request::decode(definition)?,
            upgrade_tier: Some(tier),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkey_equip_upgrade_tier_is_exact_and_capability_gated() -> Result<(), &'static str> {
        let request = Request {
            definition: NonZeroU32::new(258).ok_or("zero")?,
        };
        for tier in 0..=MAX_UPGRADE_TIER {
            let tiered = TieredRequest {
                request,
                upgrade_tier: Some(tier),
            };
            let bytes = tiered.encode().ok_or("tier")?;
            assert_eq!(bytes, [0, 0, 1, 2, tier]);
            assert_eq!(TieredRequest::decode_selected(true, &bytes), Some(tiered));
            // An old session keeps the exact four-byte payload; a fifth byte is invalid.
            assert_eq!(TieredRequest::decode_selected(false, &bytes), None);
        }
        assert_eq!(
            TieredRequest {
                request,
                upgrade_tier: Some(MAX_UPGRADE_TIER + 1)
            }
            .encode(),
            None
        );
        assert_eq!(
            TieredRequest::decode_selected(false, &request.encode()),
            Some(TieredRequest {
                request,
                upgrade_tier: None
            })
        );
        for bytes in [
            &[][..],
            &[0, 0, 1, 2][..],
            &[0, 0, 1, 2, 11][..],
            &[0, 0, 1, 2, 255][..],
            &[0, 0, 0, 0, 1][..],
            &[0, 0, 1, 2, 1, 0][..],
        ] {
            assert_eq!(TieredRequest::decode_selected(true, bytes), None);
        }
        Ok(())
    }

    #[test]
    fn hotkey_equip_canonical_definition_reference() -> Result<(), &'static str> {
        assert_eq!(
            Request {
                definition: NonZeroU32::MIN
            }
            .encode(),
            [0, 0, 0, 1]
        );
        assert_eq!(
            Request::decode(&[0, 0, 1, 0]).map(|r| r.definition.get()),
            Some(256)
        );
        for value in [1, 256, u32::MAX] {
            let definition = NonZeroU32::new(value).ok_or("zero")?;
            let request = Request { definition };
            assert_eq!(Request::decode(&request.encode()), Some(request));
        }
        for bytes in [
            &[][..],
            &[0, 0, 0][..],
            &[0, 0, 0, 0][..],
            &[0, 0, 0, 1, 0][..],
        ] {
            assert_eq!(Request::decode(bytes), None);
        }
        Ok(())
    }
}
