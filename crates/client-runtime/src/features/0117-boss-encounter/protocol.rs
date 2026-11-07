//! Capability 117 cooldown/Bosstiary projection and existing activity codecs.
pub use oteryn_session::property_services::boss_encounter::*;
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snapshot {
    pub now: u64,
    pub cooldowns: Vec<(String, u64)>,
    pub progress: Vec<(String, u64)>,
    pub points: u64,
    pub cooldown_next: Option<String>,
    pub progress_next: Option<String>,
    pub taints: Option<Taints>,
}
/// Counts are supplied by the Quest owners; the client never computes expiry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Taints {
    pub soul_war: u8,
    pub rotten_blood: u8,
    pub rotten_blood_flags: u8,
    pub rotten_blood_final: bool,
}
fn taints(bytes: &[u8]) -> Option<Taints> {
    // Four required bounded scalars, in canonical field order.
    let [8, soul, 16, rotten, 24, flags, 32, final_taint] = *bytes else {
        return None;
    };
    (soul <= 5 && rotten <= 4 && flags <= 15 && final_taint <= 1).then_some(Taints {
        soul_war: soul,
        rotten_blood: rotten,
        rotten_blood_flags: flags,
        rotten_blood_final: final_taint == 1,
    })
}
/// Opt in explicitly so servers preserve the legacy reply for older clients.
pub fn query_taints(after: &str) -> Option<super::raw_feature::FeatureCommand> {
    let mut command = query(after)?;
    command.payload.extend_from_slice(&[16, 1]);
    Some(command)
}
pub fn query(after: &str) -> Option<super::raw_feature::FeatureCommand> {
    let mut payload = Vec::new();
    if !after.is_empty() {
        if !valid_key(after) {
            return None;
        }
        payload.push(10);
        let mut len = after.len();
        while len >= 128 {
            payload.push((len as u8 & 127) | 128);
            len >>= 7;
        }
        payload.push(len as u8);
        payload.extend_from_slice(after.as_bytes());
    }
    Some(super::raw_feature::FeatureCommand {
        capability: 117,
        kind: 133,
        payload,
    })
}
fn valid_key(key: &str) -> bool {
    key.len() <= 512
        && key
            .split_once(':')
            .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty())
        && key
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b':' | b'_' | b'.' | b'-' | b'/'))
}
fn var(bytes: &[u8], at: &mut usize) -> Option<u64> {
    let mut value = 0;
    for shift in (0..70).step_by(7) {
        let byte = *bytes.get(*at)?;
        *at += 1;
        if shift == 63 && byte > 1 {
            return None;
        }
        value |= u64::from(byte & 127) << shift;
        if byte < 128 {
            return Some(value);
        }
    }
    None
}
fn take<'a>(bytes: &'a [u8], at: &mut usize) -> Option<&'a [u8]> {
    let len = usize::try_from(var(bytes, at)?).ok()?;
    let end = at.checked_add(len)?;
    let value = bytes.get(*at..end)?;
    *at = end;
    Some(value)
}
fn key(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    valid_key(text).then(|| text.to_owned())
}
fn row(bytes: &[u8]) -> Option<(String, u64)> {
    let mut at = 0;
    if var(bytes, &mut at)? != 10 {
        return None;
    }
    let key = key(take(bytes, &mut at)?)?;
    if var(bytes, &mut at)? != 16 {
        return None;
    }
    let value = var(bytes, &mut at)?;
    (at == bytes.len()).then_some((key, value))
}
pub fn decode(bytes: &[u8]) -> Option<Snapshot> {
    if bytes.len() > 34_400 {
        return None;
    }
    let mut out = Snapshot::default();
    let mut at = 0;
    let mut seen = 0_u8;
    while at < bytes.len() {
        let tag = var(bytes, &mut at)?;
        let bit = match tag {
            8 => 1,
            26 => 2,
            40 => 4,
            50 => 8,
            58 => 16,
            18 | 34 => 0,
            _ => return None,
        };
        if bit != 0 && seen & bit != 0 {
            return None;
        }
        seen |= bit;
        match tag {
            8 => out.now = var(bytes, &mut at)?,
            58 => out.taints = Some(taints(take(bytes, &mut at)?)?),
            40 => out.points = var(bytes, &mut at)?,
            26 => out.cooldown_next = Some(key(take(bytes, &mut at)?)?),
            50 => out.progress_next = Some(key(take(bytes, &mut at)?)?),
            18 | 34 => {
                let row = row(take(bytes, &mut at)?)?;
                let rows = if tag == 18 {
                    &mut out.cooldowns
                } else {
                    &mut out.progress
                };
                if rows.len() == 64 || rows.last().is_some_and(|last| last.0 >= row.0) {
                    return None;
                }
                rows.push(row);
            }
            _ => return None,
        }
    }
    if seen & 1 == 0
        || out.now > i64::MAX as u64
        || out.cooldowns.iter().any(|(_, at)| *at > i64::MAX as u64)
    {
        return None;
    }
    for (rows, next) in [
        (&out.cooldowns, &out.cooldown_next),
        (&out.progress, &out.progress_next),
    ] {
        if let Some(next) = next
            && (rows.len() != 64 || rows.last().is_none_or(|row| &row.0 != next))
        {
            return None;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boss_encounter_decoder_rejects_truncated_duplicate_and_invalid_pages() {
        assert_eq!(decode(&[8, 0, 40, 0]), Some(Snapshot::default()));
        for bytes in [
            vec![],
            vec![8],
            vec![8, 0, 8, 1],
            vec![8, 0, 26, 3, b'a', b':', b'b'],
            vec![8, 0, 18, 1, 0],
            vec![8, 0, 99],
        ] {
            assert_eq!(decode(&bytes), None);
        }
        assert!(query("not a boss key").is_none());
        assert!(query(&format!("a:{}", "x".repeat(512))).is_none());
        let mut full = vec![8, 0];
        for i in 0..64 {
            full.extend_from_slice(&[
                18,
                8,
                10,
                4,
                b'a',
                b':',
                b'0' + i / 10,
                b'0' + i % 10,
                16,
                0,
            ]);
        }
        full.extend_from_slice(&[26, 4, b'a', b':', b'6', b'3']);
        assert_eq!(
            decode(&full).and_then(|p| p.cooldown_next),
            Some("a:63".into())
        );
        full.push(0);
        assert!(decode(&full).is_none());
    }
}

pub use oteryn_session::property_services::{bosstiary_slots, hazard};
#[path = "protocol/rewards.rs"]
mod rewards;
pub use rewards::{RewardChest, decode_rewards};

#[cfg(test)]
mod taint_tests {
    use super::*;

    #[test]
    fn srx3_taints_decoder_requires_bounded_complete_unique_server_projection() {
        let value = [58, 8, 8, 5, 16, 4, 24, 15, 32, 1];
        let mut bytes = vec![8, 0];
        bytes.extend_from_slice(&value);
        assert_eq!(
            decode(&bytes).and_then(|page| page.taints),
            Some(Taints {
                soul_war: 5,
                rotten_blood: 4,
                rotten_blood_flags: 15,
                rotten_blood_final: true,
            })
        );
        for (index, invalid) in [(3, 6), (5, 5), (7, 16), (9, 2)] {
            let mut malformed = bytes.clone();
            malformed[index + 2] = invalid;
            assert!(decode(&malformed).is_none());
        }
        for end in 3..bytes.len() {
            assert!(decode(&bytes[..end]).is_none());
        }
        bytes.extend_from_slice(&value);
        assert!(decode(&bytes).is_none());
        assert_eq!(decode(&[8, 0]).and_then(|page| page.taints), None);
        assert_eq!(
            query_taints("").map(|command| command.payload),
            Some(vec![16, 1])
        );
    }
}
