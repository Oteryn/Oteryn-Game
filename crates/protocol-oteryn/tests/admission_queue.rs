use oteryn_protocol_oteryn::admission_queue::{AdmissionQueue, decode_queue, encode_queue_refusal};
use oteryn_protocol_oteryn::{
    FoundationProtocolError as Error, decode_protocol_error, decode_wire_envelope,
    encode_protocol_error,
};
#[test]
fn queue_refusal_preserves_legacy_error_and_carries_bounded_authoritative_position()
-> Result<(), Error> {
    let queue = AdmissionQueue {
        position: 3,
        retry_after_ms: 2000,
    };
    let frame = encode_queue_refusal(queue)?;
    let envelope = decode_wire_envelope(&frame)?;
    assert_eq!(decode_protocol_error(envelope.payload())?.error_code, 1115);
    assert_eq!(decode_queue(envelope.payload())?, Some(queue));
    let old = encode_protocol_error(Error::AdmissionCapacityExceeded, 0)?;
    assert_eq!(decode_queue(decode_wire_envelope(&old)?.payload())?, None);
    Ok(())
}
#[test]
fn queue_detail_rejects_bounds_duplicate_fields_and_other_refusal_codes() -> Result<(), Error> {
    for (position, retry_after_ms) in [(0, 2000), (129, 2000), (1, 0), (1, 30001)] {
        assert!(
            encode_queue_refusal(AdmissionQueue {
                position,
                retry_after_ms
            })
            .is_err()
        );
    }
    let frame = encode_queue_refusal(AdmissionQueue {
        position: 1,
        retry_after_ms: 2000,
    })?;
    let mut duplicated = decode_wire_envelope(&frame)?.payload().to_vec();
    duplicated.extend([0x80, 0x01, 1]);
    assert!(decode_queue(&duplicated).is_err());
    let other = encode_protocol_error(Error::AdmissionGrantExpired, 0)?;
    let mut other_payload = decode_wire_envelope(&other)?.payload().to_vec();
    other_payload.extend([0x80, 0x01, 1, 0x88, 0x01, 1]);
    assert!(decode_queue(&other_payload).is_err());
    Ok(())
}
