pub(super) fn envelope_size(length: usize, maximum_bytes: usize) -> Result<usize> {
    let groups = length
        .checked_div(3)
        .and_then(|groups| groups.checked_mul(4));
    let tail = match length % 3 {
        0 => 0,
        1 => 2,
        _ => 3,
    };
    let required = groups
        .and_then(|size| size.checked_add(tail))
        .and_then(|size| size.checked_add("{\"version\":1,\"payload\":\"\"}".len()))
        .ok_or(DurabilityError::InvalidStoredState)?;
    if required > maximum_bytes {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(required)
}
pub(super) fn encode_envelope(bytes: &[u8], maximum_bytes: usize) -> Result<String> {
    envelope_size(bytes.len(), maximum_bytes)?;
    let payload = URL_SAFE_NO_PAD.encode(bytes);
    checked(serde_json::to_string(&Envelope {
        version: 1,
        payload: &payload,
    }))
}
pub(super) fn decode_envelope(encoded: &str, maximum_bytes: usize) -> Result<Vec<u8>> {
    if encoded.len() > maximum_bytes {
        return Err(DurabilityError::InvalidStoredState);
    }
    // Borrowed payload parsing performs no peer-sized string copy. Unknown,
    // duplicate, escaped/noncanonical members and unsupported versions reject.
    let envelope: Envelope<'_> = checked(serde_json::from_str(encoded))?;
    if envelope.version != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    const PREFIX: &str = "{\"version\":1,\"payload\":\"";
    const SUFFIX: &str = "\"}";
    if !encoded.starts_with(PREFIX)
        || !encoded.ends_with(SUFFIX)
        || encoded.len() != PREFIX.len() + envelope.payload.len() + SUFFIX.len()
        || &encoded[PREFIX.len()..encoded.len() - SUFFIX.len()] != envelope.payload
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    // The configured engine rejects padding and nonzero unused trailing bits.
    let bytes = checked(URL_SAFE_NO_PAD.decode(envelope.payload))?;
    Ok(bytes)
}
/// Restore historical data only. Receipt restoration additionally validates the
/// original durable decided_at; neither operation creates a current source.
pub fn decode_operation(encoded: &str, maximum_bytes: usize) -> Result<FreshAdmissionOperationV1> {
    let bytes = decode_envelope(encoded, maximum_bytes)?;
    let mut reader = Reader::new(&bytes);
    let operation = read_operation(&mut reader)?;
    reader.finish()?;
    checked(operation.validate_historical(operation.transition.prepared_at))?;
    Ok(operation)
}
