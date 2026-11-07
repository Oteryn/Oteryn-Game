//! Capability387 extends command402 with an opaque current-view House target.
//! Legacy207's45-byte request remains strict and unchanged.
use crate::item_view::ItemHandle;
use std::num::NonZeroU32;
pub const CAPABILITY_HOUSE_FURNITURE_WRAP_OPAQUE_V1: u32 = 387;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpaqueWrapRequest {
    pub handle: ItemHandle,
    pub observed_definition: NonZeroU32,
}
impl OpaqueWrapRequest {
    pub fn encode(self) -> [u8; 13] {
        self.encode_marker(3)
    }
    pub fn encode_carried(self) -> [u8; 13] {
        self.encode_marker(4)
    }
    pub fn encode_probe(self) -> [u8; 13] {
        self.encode_marker(5)
    }
    fn encode_marker(self, marker: u8) -> [u8; 13] {
        let mut bytes = [0; 13];
        bytes[0] = marker;
        bytes[1..9].copy_from_slice(&self.handle.get().to_be_bytes());
        bytes[9..13].copy_from_slice(&self.observed_definition.get().to_be_bytes());
        bytes
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        Self::decode_marker(bytes, 3)
    }
    pub fn decode_carried(bytes: &[u8]) -> Option<Self> {
        Self::decode_marker(bytes, 4)
    }
    pub fn decode_probe(bytes: &[u8]) -> Option<Self> {
        Self::decode_marker(bytes, 5)
    }
    fn decode_marker(bytes: &[u8], marker: u8) -> Option<Self> {
        if bytes.len() != 13 || bytes[0] != marker {
            return None;
        }
        Some(Self {
            handle: ItemHandle::new(u64::from_be_bytes(bytes[1..9].try_into().ok()?))?,
            observed_definition: NonZeroU32::new(u32::from_be_bytes(
                bytes[9..13].try_into().ok()?,
            ))?,
        })
    }
}
pub fn encode_opaque_wrap_affordance(allowed: bool) -> [u8; 1] {
    [u8::from(allowed)]
}
pub fn decode_opaque_wrap_affordance(bytes: &[u8]) -> Option<bool> {
    match bytes {
        [0] => Some(false),
        [1] => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaque_wrap_affordance_is_an_exact_boolean() {
        for allowed in [false, true] {
            assert_eq!(
                decode_opaque_wrap_affordance(&encode_opaque_wrap_affordance(allowed)),
                Some(allowed)
            );
        }
        for bytes in [&[][..], &[2][..], &[1, 0][..]] {
            assert_eq!(decode_opaque_wrap_affordance(bytes), None);
        }
    }
    #[test]
    fn opaque_wrap_requires_exact_marker_nonzero_current_handle_and_definition() {
        for request in [
            OpaqueWrapRequest {
                handle: ItemHandle::MIN,
                observed_definition: NonZeroU32::MIN,
            },
            OpaqueWrapRequest {
                handle: ItemHandle::MAX,
                observed_definition: NonZeroU32::MAX,
            },
        ] {
            let carried = request.encode_carried();
            let probe = request.encode_probe();
            assert_eq!(OpaqueWrapRequest::decode_carried(&carried), Some(request));
            assert_eq!(OpaqueWrapRequest::decode_probe(&probe), Some(request));
            assert!(OpaqueWrapRequest::decode(&carried).is_none());
            assert!(OpaqueWrapRequest::decode_carried(&probe).is_none());
            assert!(OpaqueWrapRequest::decode_probe(&request.encode()).is_none());
            let bytes = request.encode();
            assert_eq!(OpaqueWrapRequest::decode(&bytes), Some(request));
            assert!(crate::house_wrap::Request::decode(&bytes).is_none());
            for end in 0..bytes.len() {
                assert!(OpaqueWrapRequest::decode(&bytes[..end]).is_none());
            }
            let mut bad = bytes;
            bad[0] = 1;
            assert!(OpaqueWrapRequest::decode(&bad).is_none());
            let mut bad = bytes;
            bad[1..9].fill(0);
            assert!(OpaqueWrapRequest::decode(&bad).is_none());
            let mut bad = bytes;
            bad[9..13].fill(0);
            assert!(OpaqueWrapRequest::decode(&bad).is_none());
            let mut extra = bytes.to_vec();
            extra.push(0);
            assert!(OpaqueWrapRequest::decode(&extra).is_none());
        }
    }
}
