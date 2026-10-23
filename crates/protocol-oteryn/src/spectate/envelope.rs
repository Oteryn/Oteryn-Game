use super::*;

/// Candidate outer family. Generation zero is pre-admission only. This contains no identity,
/// bearer credential or authentication assertion; Platform O2 must supply its own accepted profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObserverEnvelope {
    pub generation: u64,
    pub message: SpectateMessage,
}

impl TryFrom<&ObserverEnvelope> for Vec<u8> {
    type Error = SpectateWireError;
    fn try_from(value: &ObserverEnvelope) -> Result<Self, Self::Error> {
        let mut out = Vec::new();
        push_varint_field(&mut out, 1, OBSERVER_MESSAGE_TYPE);
        if value.generation != 0 {
            push_varint_field(&mut out, 2, value.generation);
        }
        push_message_field(&mut out, 4, &Vec::<u8>::try_from(&value.message)?);
        Ok(out)
    }
}

impl TryFrom<&[u8]> for ObserverEnvelope {
    type Error = SpectateWireError;
    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        if input.len() > crate::MAX_WIRE_FRAME_BYTES as usize {
            return Err(SpectateWireError::LimitExceeded);
        }
        let (mut kind, mut generation, mut payload) = (None, None, None);
        let mut cursor = 0;
        while cursor < input.len() {
            match read_varint(input, &mut cursor)? {
                8 => set_once(&mut kind, read_varint(input, &mut cursor)?)?,
                16 => set_once(&mut generation, read_varint(input, &mut cursor)?)?,
                34 => set_once(&mut payload, read_bytes(input, &mut cursor)?)?,
                _ => return Err(SpectateWireError::Malformed),
            }
        }
        if kind != Some(OBSERVER_MESSAGE_TYPE) {
            return Err(SpectateWireError::Malformed);
        }
        Ok(Self {
            generation: generation.unwrap_or(0),
            message: SpectateMessage::try_from(payload.ok_or(SpectateWireError::Malformed)?)?,
        })
    }
}

impl ObserverEnvelope {
    /// Classify the untrusted family without decoding/copying any admission payload.
    /// A family-15 declaration is refused even when its remaining fields are malformed.
    pub fn is_ingress(input: &[u8]) -> bool {
        let mut cursor = 0;
        while cursor < input.len() {
            let Ok(key) = crate::read_varint(input, &mut cursor) else {
                return false;
            };
            if key == 8 {
                let Ok(kind) = crate::read_varint(input, &mut cursor) else {
                    return false;
                };
                if kind == OBSERVER_MESSAGE_TYPE {
                    return true;
                }
            } else if crate::skip_field(input, &mut cursor, (key & 7) as u8).is_err() {
                return false;
            }
        }
        false
    }
}
