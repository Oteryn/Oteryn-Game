//! Svargrond arena catalogue and authoritative match wire format.
pub use crate::charm_wire::CyclopediaWireError as ArenaWireError;
use crate::charm_wire::{
    push_message_field, push_varint_field, read_bytes, read_uint32_fields, read_varint,
};
use ArenaWireError as Error;

mod match_wire;
pub use match_wire::*;

pub const CAPABILITY_ARENA_V1: u32 = 170;
pub const COMMAND_TYPE_ARENA_CATALOGUE: u32 = 240;
pub const MAX_RESULT_BYTES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaRank {
    Greenhorn,
    Scrapper,
    Warlord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaChallenge {
    pub rank: ArenaRank,
    pub fee: u32,
    pub rooms: u8,
    pub room_seconds: u16,
}

/// The catalogue request is deliberately empty: challenge facts are server-owned.
pub fn decode_catalogue_request(payload: &[u8]) -> Result<(), Error> {
    if payload.is_empty() {
        Ok(())
    } else {
        Err(Error::Malformed)
    }
}

pub const SVARGROND_CHALLENGES: [ArenaChallenge; 3] = [
    ArenaChallenge {
        rank: ArenaRank::Greenhorn,
        fee: 1_000,
        rooms: 10,
        room_seconds: 600,
    },
    ArenaChallenge {
        rank: ArenaRank::Scrapper,
        fee: 5_000,
        rooms: 10,
        room_seconds: 600,
    },
    ArenaChallenge {
        rank: ArenaRank::Warlord,
        fee: 10_000,
        rooms: 10,
        room_seconds: 600,
    },
];

pub fn encode_catalogue() -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    for challenge in SVARGROND_CHALLENGES {
        let mut value = Vec::new();
        push_varint_field(&mut value, 1, challenge.rank as u64 + 1);
        push_varint_field(&mut value, 2, u64::from(challenge.fee));
        push_varint_field(&mut value, 3, u64::from(challenge.rooms));
        push_varint_field(&mut value, 4, u64::from(challenge.room_seconds));
        push_message_field(&mut out, 1, &value);
    }
    if out.len() > MAX_RESULT_BYTES {
        Err(Error::LimitExceeded)
    } else {
        Ok(out)
    }
}

pub fn decode_catalogue(payload: &[u8]) -> Result<Vec<ArenaChallenge>, Error> {
    if payload.len() > MAX_RESULT_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut entries = Vec::new();
    let mut cursor = 0;
    while cursor < payload.len() {
        if entries.len() == 3 {
            return Err(Error::LimitExceeded);
        }
        if read_varint(payload, &mut cursor)? != 10 {
            return Err(Error::Malformed);
        }
        let [rank, fee, rooms, room_seconds] =
            read_uint32_fields::<4>(read_bytes(payload, &mut cursor)?)?;
        if rank != entries.len() as u32 + 1 || fee == 0 || rooms != 10 || room_seconds == 0 {
            return Err(Error::Malformed);
        }
        let rank = match rank {
            1 => ArenaRank::Greenhorn,
            2 => ArenaRank::Scrapper,
            3 => ArenaRank::Warlord,
            _ => return Err(Error::Malformed),
        };
        entries.push(ArenaChallenge {
            rank,
            fee,
            rooms: 10,
            room_seconds: u16::try_from(room_seconds).map_err(|_| Error::Malformed)?,
        });
    }
    if entries.len() != 3 {
        return Err(Error::Malformed);
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_svargrond_catalogue_has_three_ten_room_challenges() -> Result<(), Error> {
        assert!(decode_catalogue_request(&[]).is_ok());
        assert!(decode_catalogue_request(&[0]).is_err());
        assert_eq!(SVARGROND_CHALLENGES.len(), 3);
        assert!(
            SVARGROND_CHALLENGES
                .iter()
                .all(|challenge| challenge.rooms == 10)
        );
        let bytes = encode_catalogue()?;
        assert_eq!(decode_catalogue(&bytes)?, SVARGROND_CHALLENGES);
        // Crystal data-crystal/lib/quests/arena.lua:9 sets kickTime = 600.
        assert!(
            SVARGROND_CHALLENGES
                .iter()
                .all(|challenge| challenge.room_seconds == 600)
        );
        let mut at = 0;
        let mut entries = 0;
        while at < bytes.len() {
            assert_eq!(read_varint(&bytes, &mut at)?, 10);
            let len = read_varint(&bytes, &mut at)? as usize;
            at += len;
            entries += 1;
        }
        assert_eq!(entries, 3);
        Ok(())
    }

    #[test]
    fn arena_catalogue_decoder_refuses_partial_unknown_and_duplicate_entries() -> Result<(), Error>
    {
        assert_eq!(decode_catalogue(&[]), Err(Error::Malformed));
        assert_eq!(decode_catalogue(&[8, 1]), Err(Error::Malformed));
        assert_eq!(decode_catalogue(&[10, 5]), Err(Error::Malformed));
        let mut repeated = encode_catalogue()?;
        repeated.extend_from_slice(&encode_catalogue()?);
        assert_eq!(decode_catalogue(&repeated), Err(Error::LimitExceeded));
        assert_eq!(
            decode_catalogue(&[0; MAX_RESULT_BYTES + 1]),
            Err(Error::LimitExceeded)
        );
        Ok(())
    }
}
