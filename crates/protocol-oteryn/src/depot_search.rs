//! Bounded wire codec for searching and retrieving a character's depot holdings.
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint, set_once};
use crate::world_interaction::WorldInteractionWireError as Error;

pub const CAPABILITY_DEPOT_SEARCH_V1: u32 = 153;
pub const COMMAND_TYPE_DEPOT_SEARCH: u32 = 182;
pub const MAX_RESULTS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    Search {
        client_id: u32,
    },
    Detail {
        client_id: u32,
        tier: u8,
    },
    /// Open the list, or continue after the last id of a full page.
    Open {
        after: u32,
    },
    Retrieve {
        client_id: u32,
        all: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Found = 1,
    Retrieved = 2,
    NotFound = 3,
    NoRoom = 4,
    Rejected = 5,
    Listed = 6,
    More = 7,
    Detailed = 8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierCount {
    pub tier: u8,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub client_id: u32,
    pub depot: u32,
    pub stash: u32,
    pub inbox: u32,
    pub tiers: Vec<TierCount>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub outcome: Outcome,
    pub retrieved: u32,
    pub results: Vec<SearchResult>,
}

pub fn encode_request(value: Request) -> Result<Vec<u8>, Error> {
    let (action, client_id, all) = match value {
        Request::Search { client_id } => (1, client_id, 0),
        Request::Detail { client_id, .. } => (2, client_id, 0),
        Request::Open { after } => (4, after, 0),
        Request::Retrieve { client_id, all } => (3, client_id, u64::from(all)),
    };
    if client_id == 0 && !matches!(value, Request::Open { .. }) {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, action);
    if client_id != 0 {
        push_varint_field(&mut out, 2, u64::from(client_id));
    }
    if let Request::Detail { tier, .. } = value {
        if tier > 10 {
            return Err(Error::Malformed);
        }
        push_varint_field(&mut out, 4, u64::from(tier));
    }
    if all != 0 {
        push_varint_field(&mut out, 3, all);
    }
    Ok(out)
}

pub fn decode_request(bytes: &[u8]) -> Result<Request, Error> {
    if bytes.len() > 24 {
        return Err(Error::LimitExceeded);
    }
    let mut fields = [None; 4];
    let mut at = 0;
    while at < bytes.len() {
        let tag = read_varint(bytes, &mut at)?;
        if tag & 7 != 0 || !(1..=4).contains(&(tag >> 3)) {
            return Err(Error::Malformed);
        }
        set_once(
            &mut fields[(tag >> 3) as usize - 1],
            read_varint(bytes, &mut at)?,
        )?;
    }
    let id = u32::try_from(fields[1].unwrap_or(0)).map_err(|_| Error::Malformed)?;
    let tier = u8::try_from(fields[3].unwrap_or(0)).map_err(|_| Error::Malformed)?;
    if fields[3].is_some() && fields[0] != Some(2) {
        return Err(Error::Malformed);
    }
    let request = match (fields[0], fields[2]) {
        (Some(1), None) => Request::Search { client_id: id },
        (Some(2), None) => Request::Detail {
            client_id: id,
            tier,
        },
        (Some(4), None) => Request::Open { after: id },
        (Some(3), all @ (None | Some(0..=1))) => Request::Retrieve {
            client_id: id,
            all: all == Some(1),
        },
        _ => return Err(Error::Malformed),
    };
    encode_request(request)?;
    Ok(request)
}

pub fn encode_reply(value: &Reply) -> Result<Vec<u8>, Error> {
    if value.results.len() > MAX_RESULTS {
        return Err(Error::LimitExceeded);
    }
    if (matches!(
        value.outcome,
        Outcome::Found | Outcome::Detailed | Outcome::More
    ) && value.results.is_empty())
        || (!matches!(
            value.outcome,
            Outcome::Found | Outcome::Detailed | Outcome::Listed | Outcome::More
        ) && !value.results.is_empty())
        || (value.outcome == Outcome::More && value.results.len() != MAX_RESULTS)
        || (value.outcome == Outcome::Detailed
            && (value.results.len() != 1 || value.results[0].tiers.len() != 1))
        || (value.outcome == Outcome::Retrieved) != (value.retrieved > 0)
        || value
            .results
            .windows(2)
            .any(|r| r[0].client_id >= r[1].client_id)
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, value.outcome as u64);
    if value.retrieved != 0 {
        push_varint_field(&mut out, 2, u64::from(value.retrieved));
    }
    for result in &value.results {
        if result.client_id == 0
            || result.tiers.len() > 11
            || result.tiers.windows(2).any(|v| v[0].tier >= v[1].tier)
        {
            return Err(Error::Malformed);
        }
        let total = u64::from(result.depot) + u64::from(result.stash) + u64::from(result.inbox);
        if (total == 0 && value.outcome != Outcome::Detailed)
            || result.tiers.iter().map(|t| u64::from(t.count)).sum::<u64>() != total
        {
            return Err(Error::Malformed);
        }
        let mut row = Vec::new();
        for (field, count) in [
            (1, result.client_id),
            (2, result.depot),
            (3, result.stash),
            (4, result.inbox),
        ] {
            push_varint_field(&mut row, field, u64::from(count));
        }
        for tier in &result.tiers {
            if (tier.count == 0 && value.outcome != Outcome::Detailed) || tier.tier > 10 {
                return Err(Error::Malformed);
            }
            let mut t = Vec::new();
            push_varint_field(&mut t, 1, u64::from(tier.tier));
            push_varint_field(&mut t, 2, u64::from(tier.count));
            push_message_field(&mut row, 5, &t);
        }
        push_message_field(&mut out, 3, &row);
    }
    if out.len() > crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

pub fn decode_reply(bytes: &[u8]) -> Result<Reply, Error> {
    if bytes.len() > crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut outcome = None;
    let mut retrieved = None;
    let mut results = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let tag = read_varint(bytes, &mut at)?;
        match tag {
            8 => set_once(&mut outcome, read_varint(bytes, &mut at)?)?,
            16 => set_once(&mut retrieved, read_varint(bytes, &mut at)?)?,
            26 => {
                if results.len() == MAX_RESULTS {
                    return Err(Error::LimitExceeded);
                }
                results.push(decode_result(read_bytes(bytes, &mut at)?)?);
            }
            _ => return Err(Error::Malformed),
        }
    }
    let outcome = match outcome {
        Some(1) => Outcome::Found,
        Some(2) => Outcome::Retrieved,
        Some(3) => Outcome::NotFound,
        Some(4) => Outcome::NoRoom,
        Some(5) => Outcome::Rejected,
        Some(6) => Outcome::Listed,
        Some(7) => Outcome::More,
        Some(8) => Outcome::Detailed,
        _ => return Err(Error::Malformed),
    };
    let value = Reply {
        outcome,
        retrieved: u32::try_from(retrieved.unwrap_or(0)).map_err(|_| Error::Malformed)?,
        results,
    };
    encode_reply(&value)?;
    Ok(value)
}

fn decode_result(bytes: &[u8]) -> Result<SearchResult, Error> {
    let mut f = [None; 4];
    let mut tiers = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let tag = read_varint(bytes, &mut at)?;
        if tag == 42 {
            if tiers.len() == 11 {
                return Err(Error::LimitExceeded);
            }
            let b = read_bytes(bytes, &mut at)?;
            let mut tf = [None; 2];
            let mut p = 0;
            while p < b.len() {
                let t = read_varint(b, &mut p)?;
                if t & 7 != 0 || !(1..=2).contains(&(t >> 3)) {
                    return Err(Error::Malformed);
                }
                set_once(&mut tf[(t >> 3) as usize - 1], read_varint(b, &mut p)?)?;
            }
            tiers.push(TierCount {
                tier: u8::try_from(tf[0].unwrap_or(0)).map_err(|_| Error::Malformed)?,
                count: u32::try_from(tf[1].unwrap_or(0)).map_err(|_| Error::Malformed)?,
            });
        } else {
            if tag & 7 != 0 || !(1..=4).contains(&(tag >> 3)) {
                return Err(Error::Malformed);
            }
            set_once(
                &mut f[(tag >> 3) as usize - 1],
                read_varint(bytes, &mut at)?,
            )?;
        }
    }
    Ok(SearchResult {
        client_id: u32::try_from(f[0].unwrap_or(0)).map_err(|_| Error::Malformed)?,
        depot: u32::try_from(f[1].unwrap_or(0)).map_err(|_| Error::Malformed)?,
        stash: u32::try_from(f[2].unwrap_or(0)).map_err(|_| Error::Malformed)?,
        inbox: u32::try_from(f[3].unwrap_or(0)).map_err(|_| Error::Malformed)?,
        tiers,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn depot_open_and_tier_detail_requests_are_supported() {
        assert!(decode_request(&[8, 4]).is_ok());
        assert!(decode_request(&[8, 2, 16, 1, 32, 3]).is_ok());
    }
    #[test]
    fn strict_round_trips() {
        for request in [
            Request::Search { client_id: 3031 },
            Request::Detail {
                client_id: 3031,
                tier: 0,
            },
            Request::Open { after: 0 },
            Request::Open { after: 3031 },
            Request::Retrieve {
                client_id: 3031,
                all: true,
            },
        ] {
            assert_eq!(
                decode_request(&encode_request(request).unwrap()).unwrap(),
                request
            );
        }
        let reply = Reply {
            outcome: Outcome::Found,
            retrieved: 0,
            results: vec![SearchResult {
                client_id: 3031,
                depot: 2,
                stash: 3,
                inbox: 4,
                tiers: vec![TierCount { tier: 0, count: 9 }],
            }],
        };
        assert_eq!(decode_reply(&encode_reply(&reply).unwrap()).unwrap(), reply);
    }
    #[test]
    fn list_pages_empty_and_detail_are_distinct_on_the_wire() {
        let mut reply = Reply {
            outcome: Outcome::Listed,
            retrieved: 0,
            results: Vec::new(),
        };
        assert_eq!(decode_reply(&encode_reply(&reply).unwrap()).unwrap(), reply);
        reply.results = (1..=MAX_RESULTS as u32)
            .map(|client_id| SearchResult {
                client_id,
                depot: 100,
                stash: 200,
                inbox: 3,
                tiers: vec![TierCount {
                    tier: 0,
                    count: 303,
                }],
            })
            .collect();
        reply.outcome = Outcome::More;
        assert_eq!(decode_reply(&encode_reply(&reply).unwrap()).unwrap(), reply);
        reply.results.pop();
        assert!(encode_reply(&reply).is_err());
        reply.results.truncate(1);
        reply.outcome = Outcome::Found;
        let search = encode_reply(&reply).unwrap();
        reply.outcome = Outcome::Detailed;
        let detail = encode_reply(&reply).unwrap();
        assert_ne!(search, detail);
        assert_eq!(decode_reply(&detail).unwrap(), reply);
        reply.results[0].depot = 0;
        reply.results[0].stash = 0;
        reply.results[0].inbox = 0;
        reply.results[0].tiers[0].count = 0;
        assert_eq!(decode_reply(&encode_reply(&reply).unwrap()).unwrap(), reply);
        assert!(
            encode_request(Request::Detail {
                client_id: 1,
                tier: 11
            })
            .is_err()
        );
        assert!(decode_request(&[8, 4, 32, 0]).is_err());
    }

    #[test]
    fn rejects_invalid_and_oversize() {
        assert!(decode_request(&[0; 25]).is_err());
        assert!(encode_request(Request::Search { client_id: 0 }).is_err());
        for bytes in [
            vec![8, 1, 16, 1, 24, 0],
            vec![8, 1, 16, 1, 16, 2],
            vec![8, 3, 16, 1, 24, 2],
            // Action 4 now opens the list; action 5 remains unassigned.
            vec![8, 5, 16, 1],
            vec![8, 1, 16, 1, 32, 0],
        ] {
            assert!(decode_request(&bytes).is_err());
        }
        for reply in [
            Reply {
                outcome: Outcome::Found,
                retrieved: 0,
                results: vec![],
            },
            Reply {
                outcome: Outcome::Rejected,
                retrieved: 1,
                results: vec![],
            },
            Reply {
                outcome: Outcome::Retrieved,
                retrieved: 0,
                results: vec![],
            },
        ] {
            assert!(encode_reply(&reply).is_err());
        }
        let mut reply = Reply {
            outcome: Outcome::Found,
            retrieved: 0,
            results: vec![SearchResult {
                client_id: 3031,
                depot: 1,
                stash: 0,
                inbox: 0,
                tiers: vec![TierCount { tier: 11, count: 1 }],
            }],
        };
        assert!(encode_reply(&reply).is_err());
        reply.results[0].tiers[0].tier = 0;
        reply.results[0].tiers[0].count = 2;
        assert!(encode_reply(&reply).is_err());
        reply.results[0].tiers[0].count = 1;
        reply.results.push(reply.results[0].clone());
        assert!(encode_reply(&reply).is_err());
    }
}
