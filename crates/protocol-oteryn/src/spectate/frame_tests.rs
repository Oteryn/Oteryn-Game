#![allow(clippy::expect_used)]
use super::*;

#[test]
fn spectate_full_map_envelope_uses_native_snapshot_bound() {
    // The envelope is transparent to the native map bytes. The projection owner qualifies
    // their schema; this transport test exercises max/max+1 without copying a map fixture.
    let maximum = crate::world_map::MAX_SNAPSHOT_PAYLOAD_BYTES;
    assert!(maximum > crate::MAX_STATE_DELTA_PAYLOAD_BYTES);
    let value = ObserverEnvelope {
        generation: 1,
        message: SpectateMessage::Frame {
            domain: 17,
            payload_type: 1,
            snapshot: true,
            payload: vec![0; maximum],
        },
    };
    let bytes = Vec::<u8>::try_from(&value).expect("maximum map envelope");
    assert!(bytes.len() <= crate::MAX_WIRE_FRAME_BYTES as usize);
    assert_eq!(
        ObserverEnvelope::try_from(bytes.as_slice()).expect("decode"),
        value
    );
    for (payload_type, snapshot, length) in
        [(1, true, maximum + 1), (1, false, maximum), (2, true, 1)]
    {
        let invalid = SpectateMessage::Frame {
            domain: 17,
            payload_type,
            snapshot,
            payload: vec![0; length],
        };
        assert!(Vec::<u8>::try_from(&invalid).is_err());
    }
}
