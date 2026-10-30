//! PKG5: equip/unequip by the active generation's item definition reference.
use std::num::NonZeroU32;

pub const CAPABILITY_HOTKEY_EQUIP_V1: u32 = 179;
pub const COMMAND_TYPE_HOTKEY_EQUIP: u32 = 348;

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

#[cfg(test)]
mod tests {
    use super::*;

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
