//! The local Character's final-blow and assist credits on committed player deaths.
use crate::character_info::{MAX_PAGE_SIZE, PageRequest};

pub const COMMAND_TYPE_PLAYER_KILL_CREDITS: u32 = 6354;

pub fn decode_request(bytes: &[u8]) -> Option<PageRequest> {
    let [low, high, limit] = bytes else {
        return None;
    };
    PageRequest::new(u16::from_le_bytes([*low, *high]), *limit)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerKillCredit {
    pub occurrence: [u8; 16],
    pub victim: [u8; 16],
    pub at_unix_ms: i64,
    pub final_blow: bool,
}

pub fn encode_reply(credits: &[PlayerKillCredit]) -> Option<Vec<u8>> {
    if credits.len() > usize::from(MAX_PAGE_SIZE) {
        return None;
    }
    let mut output = vec![u8::try_from(credits.len()).ok()?];
    for credit in credits {
        if credit.victim == [0; 16] || credit.occurrence == [0; 16] || credit.at_unix_ms <= 0 {
            return None;
        }
        output.extend_from_slice(&credit.occurrence);
        output.extend_from_slice(&credit.victim);
        output.extend_from_slice(&credit.at_unix_ms.to_le_bytes());
        output.push(u8::from(credit.final_blow));
    }
    Some(output)
}

pub fn decode_reply(bytes: &[u8]) -> Option<Vec<PlayerKillCredit>> {
    let (&count, body) = bytes.split_first()?;
    let (rows, rest) = body.as_chunks::<41>();
    if count > MAX_PAGE_SIZE || rows.len() != usize::from(count) || !rest.is_empty() {
        return None;
    }
    rows.iter()
        .map(|row| {
            let occurrence = row[..16].try_into().ok()?;
            let victim = row[16..32].try_into().ok()?;
            let at_unix_ms = i64::from_le_bytes(row[32..40].try_into().ok()?);
            let final_blow = match row[40] {
                0 => false,
                1 => true,
                _ => return None,
            };
            (victim != [0; 16] && occurrence != [0; 16] && at_unix_ms > 0).then_some(
                PlayerKillCredit {
                    occurrence,
                    victim,
                    at_unix_ms,
                    final_blow,
                },
            )
        })
        .collect()
}

#[cfg(test)]
#[test]
fn occurrence_kill_credits_roundtrip_both_roles() -> Result<(), &'static str> {
    let final_hit = PlayerKillCredit {
        occurrence: [7; 16],
        victim: [9; 16],
        at_unix_ms: 1_700_000_000_000,
        final_blow: true,
    };
    for row in [final_hit, PlayerKillCredit { final_blow: false, ..final_hit }] {
        let mut wire = encode_reply(&[row]).ok_or("encode")?;
        assert_eq!(decode_reply(&wire), Some(vec![row]));
        wire[41] = 2;
        assert_eq!(decode_reply(&wire), None);
    }
    assert!(decode_request(&[0, 0, 0]).is_none());
    assert!(decode_request(&[0, 0, 51]).is_none());
    Ok(())
}
