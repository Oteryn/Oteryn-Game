//! PREMCHAN: capability-gated Game Channel list, not chat rooms. The current node owns one
//! configured Channel; this projection never invents a directory for other nodes.
use crate::charm_wire::{push_message_field, push_varint_field};

pub const CAPABILITY_CHANNEL_LIST_V1: u32 = 83;
pub const STATE_DOMAIN_CHANNEL_LIST: u32 = 48;
/// PREMCHAN-RL-01: one served Channel, two UUIDs and an explicitly encoded boolean.
pub const MAX_CHANNEL_LIST_BYTES: usize = 40;

/// The complete served-channel list (one entry). Both identities come from the Channel owner.
/// `premium_only` is always present, including false, so no flag is lost in projection.
pub fn encode_channel_list(
    world_id: &[u8; 16],
    channel_id: &[u8; 16],
    premium_only: bool,
) -> Vec<u8> {
    let mut entry = Vec::with_capacity(MAX_CHANNEL_LIST_BYTES - 2);
    push_message_field(&mut entry, 1, world_id);
    push_message_field(&mut entry, 2, channel_id);
    push_varint_field(&mut entry, 3, u64::from(premium_only));
    let mut list = Vec::with_capacity(MAX_CHANNEL_LIST_BYTES);
    push_message_field(&mut list, 1, &entry);
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premchan_list_golden_preserves_both_identities_and_explicit_boolean() {
        for premium in [false, true] {
            // Independent protobuf wire oracle: list entry length 38, two byte fields of 16,
            // then the explicitly present bool field. World and Channel must never alias.
            let mut expected = vec![0x0a, 38, 0x0a, 16];
            expected.extend_from_slice(&[0x11; 16]);
            expected.extend_from_slice(&[0x12, 16]);
            expected.extend_from_slice(&[0x22; 16]);
            expected.extend_from_slice(&[0x18, u8::from(premium)]);
            let encoded = encode_channel_list(&[0x11; 16], &[0x22; 16], premium);
            assert_eq!(encoded, expected);
            assert_eq!(encoded.len(), MAX_CHANNEL_LIST_BYTES);
        }
    }
}
