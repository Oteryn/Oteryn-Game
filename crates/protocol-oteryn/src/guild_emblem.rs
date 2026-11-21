//! GUILD-EMBLEM-1: the observer-relative guild emblem Tibia Global draws beside a player.
//! Sent only to a session that selected capability 509, as the sole optional field 8 of a
//! domain-1 type-2 visibility delta that replaces every emblem (`guild-emblem-v1.md`).
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint, set_once};
use crate::world_spatial::WorldSpatialError;
pub use crate::world_spatial_entities::EntityRef;
use crate::world_spatial_entities::{ENTITY_IDENTITY_BYTES, MAX_SNAPSHOT_ENTITIES};

pub const CAPABILITY_GUILD_EMBLEM_V1: u32 = 509;
pub const MAX_GUILD_EMBLEMS: usize = MAX_SNAPSHOT_ENTITIES;
/// One entry is at most 2 + 2 + 16 + 1 + 10 + 2 bytes.
pub const MAX_GUILD_EMBLEM_BYTES: usize = 12_288;

/// Tibia's emblem values, in the order of the five icons of its `guild.png` flag strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Emblem {
    /// Green: a member of the observer's own guild, which is at war.
    Ally = 1,
    /// Red: a member of a guild at war with the observer's guild.
    Enemy = 2,
    /// Blue: a member of another guild that is at war, not with the observer's guild.
    Neutral = 3,
    /// A member of the observer's own guild, which is at no war.
    Member = 4,
    /// A member of another guild that is at no war.
    Other = 5,
}

impl Emblem {
    pub fn from_wire(value: u64) -> Result<Self, WorldSpatialError> {
        Ok(match value {
            1 => Self::Ally,
            2 => Self::Enemy,
            3 => Self::Neutral,
            4 => Self::Member,
            5 => Self::Other,
            _ => return Err(WorldSpatialError::Malformed),
        })
    }

    /// Tibia's relation: none without a guild; otherwise by whether the viewed player's guild
    /// is at war, is the observer's own guild, and is at war with the observer's guild.
    #[must_use]
    pub const fn relation(
        has_guild: bool,
        own_guild: bool,
        guild_in_war: bool,
        at_war_with_observer: bool,
    ) -> Option<Self> {
        Some(
            match (has_guild, guild_in_war, own_guild, at_war_with_observer) {
                (false, ..) => return None,
                (true, false, true, _) => Self::Member,
                (true, false, false, _) => Self::Other,
                (true, true, true, _) => Self::Ally,
                (true, true, false, true) => Self::Enemy,
                (true, true, false, false) => Self::Neutral,
            },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildEmblem {
    pub entity: EntityRef,
    pub emblem: Emblem,
}

/// Players only, so each entity has a nonzero generation; identities are unique.
pub fn encode_guild_emblems(emblems: &[GuildEmblem]) -> Result<Vec<u8>, WorldSpatialError> {
    if emblems.len() > MAX_GUILD_EMBLEMS {
        return Err(WorldSpatialError::LimitExceeded);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for emblem in emblems {
        if emblem.entity.generation == 0 || !seen.insert(emblem.entity.identity) {
            return Err(WorldSpatialError::Malformed);
        }
        let mut entry = Vec::with_capacity(ENTITY_IDENTITY_BYTES + 16);
        push_message_field(&mut entry, 1, &emblem.entity.identity);
        push_varint_field(&mut entry, 2, emblem.entity.generation);
        push_varint_field(&mut entry, 3, emblem.emblem as u64);
        push_message_field(&mut out, 1, &entry);
    }
    if out.len() > MAX_GUILD_EMBLEM_BYTES {
        return Err(WorldSpatialError::LimitExceeded);
    }
    Ok(out)
}

pub fn decode_guild_emblems(input: &[u8]) -> Result<Vec<GuildEmblem>, WorldSpatialError> {
    if input.len() > MAX_GUILD_EMBLEM_BYTES {
        return Err(WorldSpatialError::LimitExceeded);
    }
    let malformed = |_| WorldSpatialError::Malformed;
    let mut emblems = Vec::new();
    let mut cursor = 0;
    while cursor < input.len() {
        if read_varint(input, &mut cursor).map_err(malformed)? != 10 {
            return Err(WorldSpatialError::Malformed);
        }
        if emblems.len() >= MAX_GUILD_EMBLEMS {
            return Err(WorldSpatialError::LimitExceeded);
        }
        let entry = read_bytes(input, &mut cursor).map_err(malformed)?;
        let (mut at, mut identity, mut generation, mut emblem) = (0, None, None, None);
        while at < entry.len() {
            match read_varint(entry, &mut at).map_err(malformed)? {
                10 => set_once(
                    &mut identity,
                    <[u8; ENTITY_IDENTITY_BYTES]>::try_from(
                        read_bytes(entry, &mut at).map_err(malformed)?,
                    )
                    .map_err(|_| WorldSpatialError::Malformed)?,
                )
                .map_err(malformed)?,
                16 => set_once(
                    &mut generation,
                    read_varint(entry, &mut at).map_err(malformed)?,
                )
                .map_err(malformed)?,
                24 => set_once(
                    &mut emblem,
                    Emblem::from_wire(read_varint(entry, &mut at).map_err(malformed)?)?,
                )
                .map_err(malformed)?,
                _ => return Err(WorldSpatialError::Malformed),
            }
        }
        emblems.push(GuildEmblem {
            entity: EntityRef {
                identity: identity.ok_or(WorldSpatialError::Malformed)?,
                generation: generation.ok_or(WorldSpatialError::Malformed)?,
            },
            emblem: emblem.ok_or(WorldSpatialError::Malformed)?,
        });
    }
    encode_guild_emblems(&emblems)?;
    Ok(emblems)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tibia_relation_covers_the_five_emblems() {
        assert_eq!(Emblem::relation(false, false, false, false), None);
        assert_eq!(
            Emblem::relation(true, true, false, false),
            Some(Emblem::Member)
        );
        assert_eq!(
            Emblem::relation(true, false, false, false),
            Some(Emblem::Other)
        );
        assert_eq!(
            Emblem::relation(true, true, true, false),
            Some(Emblem::Ally)
        );
        assert_eq!(
            Emblem::relation(true, false, true, true),
            Some(Emblem::Enemy)
        );
        assert_eq!(
            Emblem::relation(true, false, true, false),
            Some(Emblem::Neutral)
        );
    }

    #[test]
    fn emblem_codec_bounds_duplicates_and_unknown_values() -> Result<(), WorldSpatialError> {
        let emblem = GuildEmblem {
            entity: EntityRef {
                identity: [7; 16],
                generation: 3,
            },
            emblem: Emblem::Enemy,
        };
        let bytes = encode_guild_emblems(&[emblem])?;
        assert_eq!(decode_guild_emblems(&bytes)?, [emblem]);
        assert_eq!(decode_guild_emblems(&[])?, []);
        assert!(encode_guild_emblems(&[emblem, emblem]).is_err());
        let objects = GuildEmblem {
            entity: EntityRef {
                generation: 0,
                ..emblem.entity
            },
            ..emblem
        };
        assert!(encode_guild_emblems(&[objects]).is_err());
        assert!(decode_guild_emblems(&bytes[..bytes.len() - 1]).is_err());
        let mut unknown = bytes.clone();
        if let Some(last) = unknown.last_mut() {
            *last = 6;
        }
        assert!(decode_guild_emblems(&unknown).is_err());
        let full: Vec<_> = (0..=MAX_GUILD_EMBLEMS)
            .map(|index| GuildEmblem {
                entity: EntityRef {
                    identity: [
                        index as u8,
                        (index >> 8) as u8,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        1,
                    ],
                    generation: u64::MAX,
                },
                emblem: Emblem::Other,
            })
            .collect();
        assert_eq!(
            encode_guild_emblems(&full),
            Err(WorldSpatialError::LimitExceeded)
        );
        assert!(encode_guild_emblems(&full[..MAX_GUILD_EMBLEMS])?.len() <= MAX_GUILD_EMBLEM_BYTES);
        Ok(())
    }
}
