use super::{FoundationProtocolError, SnapshotCommitResult};
use std::collections::BTreeSet;

const MAX_PROTOBUF_FIELD_NUMBER: u64 = (1u64 << 29) - 1;

fn read_varint(input: &[u8], cursor: &mut usize) -> Result<u64, FoundationProtocolError> {
    let mut value = 0u64;
    for shift in (0..70).step_by(7) {
        let byte = *input
            .get(*cursor)
            .ok_or(FoundationProtocolError::SnapshotAssemblyInvalid)?;
        *cursor += 1;
        if shift == 63 && byte > 1 {
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(FoundationProtocolError::SnapshotAssemblyInvalid)
}

fn field_number(key: u64) -> Result<u32, FoundationProtocolError> {
    let raw = key >> 3;
    if raw == 0 || raw > MAX_PROTOBUF_FIELD_NUMBER {
        return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
    }
    u32::try_from(raw).map_err(|_| FoundationProtocolError::SnapshotAssemblyInvalid)
}

fn length_delimited<'a>(
    input: &'a [u8],
    cursor: &mut usize,
) -> Result<&'a [u8], FoundationProtocolError> {
    let length = usize::try_from(read_varint(input, cursor)?)
        .map_err(|_| FoundationProtocolError::SnapshotAssemblyInvalid)?;
    let end = cursor
        .checked_add(length)
        .filter(|end| *end <= input.len())
        .ok_or(FoundationProtocolError::SnapshotAssemblyInvalid)?;
    let value = input
        .get(*cursor..end)
        .ok_or(FoundationProtocolError::SnapshotAssemblyInvalid)?;
    *cursor = end;
    Ok(value)
}

fn skip_field(
    input: &[u8],
    cursor: &mut usize,
    wire: u8,
) -> Result<(), FoundationProtocolError> {
    let width = match wire {
        0 => {
            read_varint(input, cursor)?;
            return Ok(());
        }
        1 => 8,
        2 => usize::try_from(read_varint(input, cursor)?)
            .map_err(|_| FoundationProtocolError::SnapshotAssemblyInvalid)?,
        5 => 4,
        _ => return Err(FoundationProtocolError::SnapshotAssemblyInvalid),
    };
    *cursor = cursor
        .checked_add(width)
        .filter(|end| *end <= input.len())
        .ok_or(FoundationProtocolError::SnapshotAssemblyInvalid)?;
    Ok(())
}

fn state_domain_id(input: &[u8]) -> Result<u32, FoundationProtocolError> {
    let mut cursor = 0usize;
    let mut domain_id = None;
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        let field = field_number(key)?;
        let wire = (key & 7) as u8;
        match (field, wire) {
            (1, 0) if domain_id.is_none() => {
                let raw = read_varint(input, &mut cursor)?;
                let value = u32::try_from(raw)
                    .map_err(|_| FoundationProtocolError::SnapshotAssemblyInvalid)?;
                if value == 0 {
                    return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
                }
                domain_id = Some(value);
            }
            (2 | 3, 0) => {
                read_varint(input, &mut cursor)?;
            }
            (4, 2) => {
                length_delimited(input, &mut cursor)?;
            }
            (1..=4, _) => return Err(FoundationProtocolError::SnapshotAssemblyInvalid),
            (_, unknown_wire) => skip_field(input, &mut cursor, unknown_wire)?,
        }
    }
    domain_id.ok_or(FoundationProtocolError::SnapshotAssemblyInvalid)
}

fn validate_snapshot_body(input: &[u8]) -> Result<(), FoundationProtocolError> {
    let mut cursor = 0usize;
    let mut domain_count = 0usize;
    let mut seen_domains = BTreeSet::new();
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        let field = field_number(key)?;
        let wire = (key & 7) as u8;
        match (field, wire) {
            (1, 2) => {
                domain_count = domain_count
                    .checked_add(1)
                    .ok_or(FoundationProtocolError::SnapshotLimitExceeded)?;
                if domain_count > super::protocol::MAX_STATE_DOMAINS_PER_SYNC {
                    return Err(FoundationProtocolError::SnapshotLimitExceeded);
                }
                let domain = length_delimited(input, &mut cursor)?;
                if !seen_domains.insert(state_domain_id(domain)?) {
                    return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
                }
            }
            (1, _) => return Err(FoundationProtocolError::SnapshotAssemblyInvalid),
            (_, unknown_wire) => skip_field(input, &mut cursor, unknown_wire)?,
        }
    }
    Ok(())
}

/// Public snapshot assembly boundary. The private wire assembler verifies chunk
/// ordering/size/generation; this facade additionally validates the assembled
/// SnapshotBody state-domain collection before exposing the body to consumers.
#[derive(Debug, Clone, Default)]
pub struct SnapshotBarrier {
    core: super::protocol::SnapshotBarrier,
}

impl SnapshotBarrier {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.core.is_active()
    }

    pub fn begin(
        &mut self,
        id: u64,
        chunk_count: u32,
        total_bytes: u64,
        target_sequence: u64,
        generation: u64,
    ) -> Result<(), FoundationProtocolError> {
        self.core
            .begin(id, chunk_count, total_bytes, target_sequence, generation)
    }

    pub fn chunk(
        &mut self,
        id: u64,
        index: u32,
        data: &[u8],
        generation: u64,
    ) -> Result<(), FoundationProtocolError> {
        self.core.chunk(id, index, data, generation)
    }

    pub fn commit(
        &mut self,
        id: u64,
        generation: u64,
    ) -> Result<SnapshotCommitResult, FoundationProtocolError> {
        let result = self.core.commit(id, generation)?;
        validate_snapshot_body(result.body())?;
        Ok(result)
    }

    #[must_use]
    pub fn may_emit_sequenced(&self, sequence: u64, generation: u64) -> bool {
        self.core.may_emit_sequenced(sequence, generation)
    }

    pub fn discard_for_generation_change(&mut self) {
        self.core.discard_for_generation_change();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn varint(mut value: usize) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if value == 0 {
                return out;
            }
        }
    }

    fn domain(domain_id: usize) -> Vec<u8> {
        let mut nested = vec![0x08];
        nested.extend(varint(domain_id));
        nested.extend([0x10, 0x01, 0x18, 0x01, 0x22, 0x00]);
        nested
    }

    fn body(ids: impl IntoIterator<Item = usize>) -> Vec<u8> {
        let mut body = Vec::new();
        for id in ids {
            let nested = domain(id);
            body.push(0x0a);
            body.extend(varint(nested.len()));
            body.extend(nested);
        }
        body
    }

    fn commit_body(body: &[u8]) -> Result<SnapshotCommitResult, FoundationProtocolError> {
        let mut barrier = SnapshotBarrier::new();
        barrier.begin(1, 1, body.len() as u64, 10, 1)?;
        barrier.chunk(1, 0, body, 1)?;
        barrier.commit(1, 1)
    }

    #[test]
    fn snapshot_body_accepts_256_unique_domains() -> Result<(), FoundationProtocolError> {
        let body = body(1..=256);
        assert_eq!(commit_body(&body)?.body(), body);
        Ok(())
    }

    #[test]
    fn snapshot_body_rejects_duplicate_domains() {
        let body = body([7, 7]);
        assert_eq!(
            commit_body(&body),
            Err(FoundationProtocolError::SnapshotAssemblyInvalid)
        );
    }

    #[test]
    fn snapshot_body_preserves_additive_unknown_fields() -> Result<(), FoundationProtocolError> {
        let mut body = body([7]);
        body.extend([0x10, 0x01]);
        assert_eq!(commit_body(&body)?.body(), body);
        Ok(())
    }
}