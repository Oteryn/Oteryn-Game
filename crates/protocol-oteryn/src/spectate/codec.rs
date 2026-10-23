use super::*;
type Result<T> = std::result::Result<T, SpectateWireError>;

impl TryFrom<&SpectateMessage> for Vec<u8> {
    type Error = SpectateWireError;
    fn try_from(value: &SpectateMessage) -> Result<Self> {
        let mut out = Vec::new();
        let kind = match value {
            SpectateMessage::GrantSet(_) => 1,
            SpectateMessage::GrantRevoke => 2,
            SpectateMessage::TargetState { .. } => 3,
            SpectateMessage::Stop => 4,
            SpectateMessage::Frame { .. } => 5,
            SpectateMessage::End(_) => 6,
            SpectateMessage::NotAvailable => 7,
            SpectateMessage::NotAllowed => 8,
            SpectateMessage::Keepalive => 9,
        };
        push_varint_field(&mut out, 1, kind);
        match value {
            SpectateMessage::GrantSet(audience) => push_varint_field(&mut out, 2, *audience as u64),
            SpectateMessage::TargetState { active, count } => {
                if *count > 50 || (!active && *count != 0) {
                    return Err(SpectateWireError::Malformed);
                }
                push_varint_field(&mut out, 3, u64::from(*active));
                push_varint_field(&mut out, 4, u64::from(*count));
            }
            SpectateMessage::Frame {
                domain,
                payload_type,
                snapshot,
                payload,
            } => {
                if frame_limit(*domain, *payload_type, *snapshot)
                    .is_none_or(|maximum| payload.len() > maximum)
                {
                    return Err(SpectateWireError::Malformed);
                }
                push_varint_field(&mut out, 5, u64::from(*domain));
                push_varint_field(&mut out, 6, u64::from(*payload_type));
                push_message_field(&mut out, 7, payload);
                push_varint_field(&mut out, 9, u64::from(*snapshot));
            }
            SpectateMessage::End(reason) => push_varint_field(&mut out, 8, *reason as u64),
            _ => {}
        }
        Ok(out)
    }
}

impl TryFrom<&[u8]> for SpectateMessage {
    type Error = SpectateWireError;
    fn try_from(input: &[u8]) -> Result<Self> {
        if input.len() > crate::world_map::MAX_SNAPSHOT_PAYLOAD_BYTES + 64 {
            return Err(SpectateWireError::LimitExceeded);
        }
        let mut fields = [None; 9];
        let mut payload = None;
        let mut cursor = 0;
        while cursor < input.len() {
            let key = read_varint(input, &mut cursor)?;
            if key == 58 {
                set_once(&mut payload, read_bytes(input, &mut cursor)?)?;
            } else if key & 7 == 0 && (1..=9).contains(&(key >> 3)) && key >> 3 != 7 {
                set_once(
                    &mut fields[(key >> 3) as usize - 1],
                    read_varint(input, &mut cursor)?,
                )?;
            } else {
                return Err(SpectateWireError::Malformed);
            }
        }
        let [
            kind,
            audience,
            active,
            count,
            domain,
            payload_type,
            _,
            reason,
            snapshot,
        ] = fields;
        let value = match (
            kind,
            audience,
            active,
            count,
            domain,
            payload_type,
            payload,
            reason,
            snapshot,
        ) {
            (Some(1), Some(a), None, None, None, None, None, None, None) => {
                Self::GrantSet(match a {
                    1 => Audience::Code,
                    2 => Audience::Contacts,
                    _ => return Err(SpectateWireError::Malformed),
                })
            }
            (Some(2), None, None, None, None, None, None, None, None) => Self::GrantRevoke,
            (Some(3), None, Some(a @ 0..=1), Some(n @ 0..=50), None, None, None, None, None)
                if a == 1 || n == 0 =>
            {
                Self::TargetState {
                    active: a == 1,
                    count: n as u8,
                }
            }
            (Some(4), None, None, None, None, None, None, None, None) => Self::Stop,
            (
                Some(5),
                None,
                None,
                None,
                Some(d @ (1 | 12 | 17 | 19)),
                Some(t),
                Some(p),
                None,
                Some(s @ 0..=1),
            ) if t <= u64::from(u32::MAX)
                && frame_limit(d as u32, t as u32, s == 1)
                    .is_some_and(|maximum| p.len() <= maximum) =>
            {
                Self::Frame {
                    domain: d as u32,
                    payload_type: t as u32,
                    snapshot: s == 1,
                    payload: p.to_vec(),
                }
            }
            (Some(6), None, None, None, None, None, None, Some(r), None) => {
                Self::End(r.try_into()?)
            }
            (Some(7), None, None, None, None, None, None, None, None) => Self::NotAvailable,
            (Some(8), None, None, None, None, None, None, None, None) => Self::NotAllowed,
            (Some(9), None, None, None, None, None, None, None, None) => Self::Keepalive,
            _ => return Err(SpectateWireError::Malformed),
        };
        Ok(value)
    }
}

// Reuse each native payload's registered bound. In particular a full map snapshot is
// larger than FND02's ordinary StateDelta limit; it remains below the outer frame limit.
fn frame_limit(domain: u32, payload_type: u32, snapshot: bool) -> Option<usize> {
    match (domain, payload_type, snapshot) {
        (1, 2, _) => Some(crate::world_spatial_entities::MAX_WORLD_SPATIAL_ENTITIES_PAYLOAD_BYTES),
        (12, 1, false) => Some(crate::chat::MAX_CHAT_LINE_BYTES),
        (17, 1, true) => Some(crate::world_map::MAX_SNAPSHOT_PAYLOAD_BYTES),
        (17, 1, false) => Some(crate::world_map::MAX_DELTA_PAYLOAD_BYTES),
        (19, 1, false) => Some(crate::presentation::MAX_PRESENTATION_BATCH_BYTES),
        _ => None,
    }
}
