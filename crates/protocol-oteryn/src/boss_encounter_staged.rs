//! Boss activity intents and typed outcomes. Neither is runtime authority.
use super::{
    Error, MAX_QUERY_BYTES, push_message_field, push_varint_field, read_bytes, read_varint,
    valid_key,
};

/// The client identifies only admitted content and the lever use occurrence. Party membership,
/// positions, levels, cooldowns and room coordinates are deliberately server-resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossLeverRequest {
    pub boss_key: String,
}

pub fn decode_boss_key(payload: &[u8]) -> Result<BossLeverRequest, Error> {
    if payload.len() > MAX_QUERY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut at = 0;
    if read_varint(payload, &mut at)? != 10 {
        return Err(Error::Malformed);
    }
    let key = std::str::from_utf8(read_bytes(payload, &mut at)?).map_err(|_| Error::Malformed)?;
    if at != payload.len() || !valid_key(key) {
        return Err(Error::Malformed);
    }
    Ok(BossLeverRequest {
        boss_key: key.into(),
    })
}

pub fn encode_boss_key(key: &str) -> Result<Vec<u8>, Error> {
    if !valid_key(key) {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, key.as_bytes());
    Ok(out)
}

/// Opaque fresh-instance receipt. It is presentation data, never an admission credential.
pub fn encode_instance_receipt(
    instance: &[u8; 16],
    deadline_unix_ms: u64,
) -> Result<Vec<u8>, Error> {
    if deadline_unix_ms > i64::MAX as u64 {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, instance);
    push_varint_field(&mut out, 2, deadline_unix_ms);
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BossActivityRefusal {
    InvalidInput = 1,
    AuthorityRejected = 2,
    NoCapacity = 3,
    ContentUnavailable = 4,
    ContentChanged = 5,
    ReplayConflict = 6,
    NotDue = 7,
    AlreadyReturned = 8,
    OwnerStillCurrent = 9,
    PartyCount = 10,
    Position = 11,
    Level = 12,
    Cooldown = 13,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BossActivityReply {
    /// The exact destination identity follows in the ACTIVITY_SCOPE barrier.
    Accepted {
        handoff: [u8; 16],
        deadline_unix_ms: Option<u64>,
    },
    Refused(BossActivityRefusal),
}

pub fn encode_boss_activity_reply(reply: &BossActivityReply) -> Result<Vec<u8>, Error> {
    let mut output = Vec::new();
    match reply {
        BossActivityReply::Accepted {
            handoff,
            deadline_unix_ms,
        } => {
            if handoff[6] >> 4 != 7
                || handoff[8] & 0xc0 != 0x80
                || deadline_unix_ms.is_some_and(|value| value > i64::MAX as u64)
            {
                return Err(Error::Malformed);
            }
            push_varint_field(&mut output, 1, 1);
            push_message_field(&mut output, 2, handoff);
            if let Some(deadline) = deadline_unix_ms {
                push_varint_field(&mut output, 3, *deadline);
            }
        }
        BossActivityReply::Refused(reason) => {
            push_varint_field(&mut output, 1, 2);
            push_varint_field(&mut output, 4, *reason as u64);
        }
    }
    Ok(output)
}

pub fn decode_boss_activity_reply(body: &[u8]) -> Result<BossActivityReply, Error> {
    use crate::charm_wire::set_once;
    if body.len() > 64 {
        return Err(Error::LimitExceeded);
    }
    let (mut at, mut status, mut handoff, mut deadline, mut refusal) = (0, None, None, None, None);
    while at < body.len() {
        match read_varint(body, &mut at)? {
            8 => set_once(&mut status, read_varint(body, &mut at)?)?,
            18 => set_once(
                &mut handoff,
                <[u8; 16]>::try_from(read_bytes(body, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            24 => set_once(&mut deadline, read_varint(body, &mut at)?)?,
            32 => set_once(&mut refusal, read_varint(body, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    let result = match (status, handoff, deadline, refusal) {
        (Some(1), Some(handoff), deadline_unix_ms, None) => BossActivityReply::Accepted {
            handoff,
            deadline_unix_ms,
        },
        (Some(2), None, None, Some(value)) => BossActivityReply::Refused(match value {
            1 => BossActivityRefusal::InvalidInput,
            2 => BossActivityRefusal::AuthorityRejected,
            3 => BossActivityRefusal::NoCapacity,
            4 => BossActivityRefusal::ContentUnavailable,
            5 => BossActivityRefusal::ContentChanged,
            6 => BossActivityRefusal::ReplayConflict,
            7 => BossActivityRefusal::NotDue,
            8 => BossActivityRefusal::AlreadyReturned,
            9 => BossActivityRefusal::OwnerStillCurrent,
            10 => BossActivityRefusal::PartyCount,
            11 => BossActivityRefusal::Position,
            12 => BossActivityRefusal::Level,
            13 => BossActivityRefusal::Cooldown,
            _ => return Err(Error::Malformed),
        }),
        _ => return Err(Error::Malformed),
    };
    encode_boss_activity_reply(&result)?;
    Ok(result)
}

#[cfg(test)]
mod activity_tests {
    use super::*;
    #[test]
    fn content_unavailable_and_no_capacity_are_distinct_typed_outcomes() -> Result<(), Error> {
        for reason in [
            BossActivityRefusal::ContentUnavailable,
            BossActivityRefusal::NoCapacity,
        ] {
            let value = BossActivityReply::Refused(reason);
            assert_eq!(
                decode_boss_activity_reply(&encode_boss_activity_reply(&value)?),
                Ok(value)
            );
        }
        assert_ne!(
            encode_boss_activity_reply(&BossActivityReply::Refused(
                BossActivityRefusal::ContentUnavailable
            )),
            encode_boss_activity_reply(&BossActivityReply::Refused(
                BossActivityRefusal::NoCapacity
            ))
        );
        Ok(())
    }
    #[test]
    fn a_client_cannot_add_authoritative_positions_or_generations_to_a_lever_key()
    -> Result<(), Error> {
        let mut key = encode_boss_key("oteryn:content.monster.boss")?;
        push_varint_field(&mut key, 2, 400);
        assert!(decode_boss_key(&key).is_err());
        assert!(decode_boss_activity_reply(&[8, 2, 32, 3, 24, 1]).is_err());
        Ok(())
    }
}
