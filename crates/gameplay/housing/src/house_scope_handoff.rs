//! Pure house identity, entry refusal and tile bounds (SCOPE-HANDOFF-1).

use oteryn_gameplay_domain::domain::WorldId;
use sha2::{Digest, Sha256};

/// HOUSERT0-RL-03: Characters inside one house, PREPARED entries included.
pub const HOUSE_SCOPE_CHARACTERS_MAX: u64 = 200;
/// Maximum UTF-8 bytes of a house key (migration 0025).
pub const HOUSE_KEY_MAX_BYTES: usize = 512;
/// Maximum bytes of an encoded HouseInterior spatial position (migration 0025).
pub const HOUSE_TILE_MAX_BYTES: usize = 128;

const HOUSE_KEY_PREFIX: &str = "oteryn:content.house.";
const HOUSE_INSTANCE_DOMAIN: &[u8] = b"oteryn:house-scope-instance:v1";
/// Exact identity of one house: its World and its content house key (migration 0025).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HouseId {
    world_id: WorldId,
    house_key: String,
}

impl HouseId {
    /// `None` unless the key is `oteryn:content.house.[a-z0-9_]+` within 512 bytes.
    #[must_use]
    pub fn new(world_id: WorldId, house_key: &str) -> Option<Self> {
        let suffix = house_key.strip_prefix(HOUSE_KEY_PREFIX)?;
        let valid = house_key.len() <= HOUSE_KEY_MAX_BYTES
            && !suffix.is_empty()
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        valid.then(|| Self {
            world_id,
            house_key: house_key.to_owned(),
        })
    }

    #[must_use]
    pub const fn world_id(&self) -> WorldId {
        self.world_id
    }

    #[must_use]
    pub fn house_key(&self) -> &str {
        &self.house_key
    }

    /// The tagged assignment key: `\x02 || world || house key`.
    #[must_use]
    pub fn scope_key(&self) -> Vec<u8> {
        let mut key = Vec::with_capacity(17 + self.house_key.len());
        key.push(0x02);
        key.extend_from_slice(self.world_id.as_bytes());
        key.extend_from_slice(self.house_key.as_bytes());
        key
    }

    /// The UUIDv7-shaped instance id of this house; equal to `game_house_scope_instance_id`.
    #[must_use]
    pub fn instance_id(&self) -> [u8; 16] {
        let digest = Sha256::new()
            .chain_update(HOUSE_INSTANCE_DOMAIN)
            .chain_update(self.world_id.as_bytes())
            .chain_update(self.house_key.as_bytes())
            .finalize();
        let mut id = [0_u8; 16];
        id.copy_from_slice(&digest[..16]);
        id[6] = (id[6] & 0x0f) | 0x70;
        id[8] = (id[8] & 0x3f) | 0x80;
        id
    }
}

/// Typed refusals of a house entry (HOUSE-RUNTIME-0 §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseEntryRefusal {
    NoAccess,
    InCombat,
    Busy,
    HouseClosed,
    NoRoom,
}

impl HouseEntryRefusal {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoAccess => "NO_ACCESS",
            Self::InCombat => "IN_COMBAT",
            Self::Busy => "BUSY",
            Self::HouseClosed => "HOUSE_CLOSED",
            Self::NoRoom => "NO_ROOM",
        }
    }
}

/// Direction of a handoff. Only `Entry` is admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseHandoffDirection {
    Entry,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseHandoffState {
    Prepared,
    /// Committed; the entry tile stays reserved until the destination places the Character.
    Committed,
    /// Committed and the entry tile released; retained history.
    Released,
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn world() -> std::result::Result<WorldId, Box<dyn std::error::Error>> {
        WorldId::from_bytes([42, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 42])
            .map_err(|error| format!("{error:?}").into())
    }

    #[test]
    fn house_ids_follow_the_content_key_grammar() -> TestResult {
        let world = world()?;
        assert!(HouseId::new(world, "oteryn:content.house.thais_1").is_some());
        assert!(HouseId::new(world, "oteryn:content.house.").is_none());
        assert!(HouseId::new(world, "oteryn:content.house.Thais").is_none());
        assert!(HouseId::new(world, "oteryn:content.npc.thais").is_none());
        let long = format!("{HOUSE_KEY_PREFIX}{}", "a".repeat(HOUSE_KEY_MAX_BYTES));
        assert!(HouseId::new(world, &long).is_none());
        Ok(())
    }

    #[test]
    fn house_scope_identity_is_tagged_and_v7_shaped() -> TestResult {
        let world = world()?;
        let house = HouseId::new(world, "oteryn:content.house.thais_1").ok_or("house")?;
        let key = house.scope_key();
        assert_eq!(key[0], 0x02);
        assert_eq!(&key[1..17], world.as_bytes());
        let instance = house.instance_id();
        assert_eq!(instance[6] >> 4, 7);
        assert_eq!(instance[8] & 0xc0, 0x80);
        assert_eq!(instance, house.instance_id());
        let other = HouseId::new(world, "oteryn:content.house.thais_2").ok_or("house")?;
        assert_ne!(instance, other.instance_id());
        Ok(())
    }

    #[test]
    fn refusals_use_their_wire_names() {
        assert_eq!(HouseEntryRefusal::NoAccess.as_str(), "NO_ACCESS");
        assert_eq!(HouseEntryRefusal::InCombat.as_str(), "IN_COMBAT");
        assert_eq!(HouseEntryRefusal::Busy.as_str(), "BUSY");
        assert_eq!(HouseEntryRefusal::HouseClosed.as_str(), "HOUSE_CLOSED");
        assert_eq!(HouseEntryRefusal::NoRoom.as_str(), "NO_ROOM");
    }
}
