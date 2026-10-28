//! Svargrond arena catalogue wire format.
use crate::charm_wire::{CyclopediaWireError as Error, push_message_field, push_varint_field};

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
        room_seconds: 120,
    },
    ArenaChallenge {
        rank: ArenaRank::Scrapper,
        fee: 5_000,
        rooms: 10,
        room_seconds: 120,
    },
    ArenaChallenge {
        rank: ArenaRank::Warlord,
        fee: 10_000,
        rooms: 10,
        room_seconds: 120,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::charm_wire::read_varint;

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
}
