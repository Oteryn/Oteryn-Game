//! Optional N8 capacity queue metadata; old peers skip fields 16/17.
use super::{
    FoundationProtocolError as Error, ProtocolDisposition, decode_field_number,
    decode_protocol_error, push_bytes, push_scalar, read_singular_varint, read_varint, skip_field,
};
pub const MAX_QUEUE_POSITION: u32 = 128;
pub const MAX_RETRY_AFTER_MS: u32 = 30_000;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionQueue {
    pub position: u32,
    pub retry_after_ms: u32,
}
impl AdmissionQueue {
    fn validate(self) -> Result<Self, Error> {
        if self.position == 0
            || self.position > MAX_QUEUE_POSITION
            || self.retry_after_ms == 0
            || self.retry_after_ms > MAX_RETRY_AFTER_MS
        {
            return Err(Error::MalformedEnvelope);
        }
        Ok(self)
    }
}
pub fn encode_queue_refusal(queue: AdmissionQueue) -> Result<Vec<u8>, Error> {
    let queue = queue.validate()?;
    Ok(capacity_frame(Some(queue)))
}

pub fn encode_channel_full() -> Vec<u8> {
    capacity_frame(None)
}

fn capacity_frame(queue: Option<AdmissionQueue>) -> Vec<u8> {
    let mut payload = Vec::new();
    push_scalar(
        &mut payload,
        1,
        u64::from(Error::AdmissionCapacityExceeded.code()),
    );
    push_scalar(&mut payload, 2, ProtocolDisposition::TransportFatal as u64);
    if let Some(queue) = queue {
        push_scalar(&mut payload, 16, u64::from(queue.position));
        push_scalar(&mut payload, 17, u64::from(queue.retry_after_ms));
    } else {
        push_scalar(
            &mut payload,
            18,
            u64::from(super::channel_switch::SwitchLock::ChannelFull.code()),
        );
    }
    let mut frame = vec![8, 14];
    push_bytes(&mut frame, 4, &payload);
    frame
}
pub fn decode_queue(payload: &[u8]) -> Result<Option<AdmissionQueue>, Error> {
    Ok(match decode_capacity(payload)? {
        Some(CapacityRefusal::Queued(queue)) => Some(queue),
        _ => None,
    })
}

pub enum CapacityRefusal {
    Queued(AdmissionQueue),
    ChannelFull,
}

pub fn decode_capacity(payload: &[u8]) -> Result<Option<CapacityRefusal>, Error> {
    let base = decode_protocol_error(payload)?;
    let mut fields = [None; 3];
    let mut cursor = 0;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match field {
            16..=18 => {
                read_singular_varint(payload, &mut cursor, wire, &mut fields[field as usize - 16])?
            }
            _ => skip_field(payload, &mut cursor, wire)?,
        }
    }
    if fields == [None; 3] {
        return Ok(None);
    }
    if base.error_code != Error::AdmissionCapacityExceeded.code()
        || base.disposition != ProtocolDisposition::TransportFatal
        || base.related_command_id != 0
        || base.expected_command_id != 0
        || base.expected_server_sequence != 0
    {
        return Err(Error::MalformedEnvelope);
    }
    if fields
        == [
            None,
            None,
            Some(u64::from(
                super::channel_switch::SwitchLock::ChannelFull.code(),
            )),
        ]
    {
        return Ok(Some(CapacityRefusal::ChannelFull));
    }
    let [Some(position), Some(retry_after_ms), None] = fields else {
        return Err(Error::MalformedEnvelope);
    };
    let queue = AdmissionQueue {
        position: u32::try_from(position).map_err(|_| Error::MalformedEnvelope)?,
        retry_after_ms: u32::try_from(retry_after_ms).map_err(|_| Error::MalformedEnvelope)?,
    }
    .validate()?;
    Ok(Some(CapacityRefusal::Queued(queue)))
}
