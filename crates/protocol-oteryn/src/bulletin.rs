//! Optional read-only operator bulletin on authenticated ServerAccepted (field 21).
use super::{
    FoundationProtocolError as Error, MessageType, bounded_length_delimited, decode_field_number,
    decode_wire_envelope, push_bytes, read_varint, skip_field,
};
pub const MAX_DOCUMENT_BYTES: usize = 256 * 1024;
fn valid(document: &str) -> bool {
    !document.is_empty() && document.len() <= MAX_DOCUMENT_BYTES && !document.contains('\0')
}
/// Existing acceptance identity/capabilities remain unchanged; old readers skip field 21.
pub fn append_to_acceptance(frame: &[u8], document: &str) -> Result<Vec<u8>, Error> {
    let envelope = decode_wire_envelope(frame)?;
    if envelope.message_type() != MessageType::ServerAccepted || !valid(document) {
        return Err(Error::MalformedEnvelope);
    }
    super::decode_server_accepted(envelope.payload())?;
    let mut payload = envelope.payload().to_vec();
    if document_from_acceptance(&payload)?.is_some() {
        return Err(Error::MalformedEnvelope);
    }
    push_bytes(&mut payload, 21, document.as_bytes());
    let mut output = vec![8, 2];
    push_bytes(&mut output, 4, &payload);
    Ok(output)
}
/// The JSON news/events schema is validated by the publication source and presentation caller.
pub fn document_from_acceptance(payload: &[u8]) -> Result<Option<&str>, Error> {
    let mut cursor = 0;
    let mut document = None;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        if field == 21 {
            if wire != 2 || document.is_some() {
                return Err(Error::MalformedEnvelope);
            }
            let bytes = bounded_length_delimited(
                payload,
                &mut cursor,
                MAX_DOCUMENT_BYTES,
                Error::BootstrapLimitExceeded,
            )?;
            let text = std::str::from_utf8(bytes).map_err(|_| Error::MalformedEnvelope)?;
            if !valid(text) {
                return Err(Error::MalformedEnvelope);
            }
            document = Some(text);
        } else {
            skip_field(payload, &mut cursor, wire)?;
        }
    }
    Ok(document)
}
