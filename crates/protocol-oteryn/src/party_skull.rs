//! PARTY-SKULL-1: observer-relative icons. Only runtime incarnations cross the wire.
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint, set_once};
use crate::party::PartyWireError;
use crate::world_spatial_entities::EntityRef;

pub const CAPABILITY_PARTY_SKULL_V1: u32 = 134;
pub const COMMAND_TYPE_PARTY_SKULL_QUERY: u32 = 155;
pub const MAX_PARTY_ICON_BYTES: usize = 12_288;
pub const MAX_PARTY_ICONS: usize = 256;

/// Shield values match the pinned sources: none, invitation blue/yellow, member,
/// leader, shared member/leader, inactive blink member/leader, inactive member/leader,
/// outsider gray. Skull: none/yellow/green/white/red/black/orange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartyIcon {
    pub entity: EntityRef,
    pub shield: u8,
    pub skull: u8,
}

pub fn encode_party_icons(icons: &[PartyIcon]) -> Result<Vec<u8>, PartyWireError> {
    if icons.len() > MAX_PARTY_ICONS {
        return Err(PartyWireError::LimitExceeded);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for icon in icons {
        if icon.entity.generation == 0
            || icon.shield > 11
            || icon.skull > 6
            || !seen.insert(icon.entity.identity)
        {
            return Err(PartyWireError::Malformed);
        }
        let mut entry = Vec::new();
        push_message_field(&mut entry, 1, &icon.entity.identity);
        push_varint_field(&mut entry, 2, icon.entity.generation);
        push_varint_field(&mut entry, 3, u64::from(icon.shield));
        push_varint_field(&mut entry, 4, u64::from(icon.skull));
        push_message_field(&mut out, 1, &entry);
    }
    if out.len() > MAX_PARTY_ICON_BYTES {
        return Err(PartyWireError::LimitExceeded);
    }
    Ok(out)
}

pub fn decode_party_icons(input: &[u8]) -> Result<Vec<PartyIcon>, PartyWireError> {
    if input.len() > MAX_PARTY_ICON_BYTES {
        return Err(PartyWireError::LimitExceeded);
    }
    let mut icons = Vec::new();
    let mut cursor = 0;
    while cursor < input.len() {
        if read_varint(input, &mut cursor)? != 10 || icons.len() >= MAX_PARTY_ICONS {
            return Err(PartyWireError::Malformed);
        }
        let entry = read_bytes(input, &mut cursor)?;
        let (mut at, mut identity, mut generation, mut shield, mut skull) =
            (0, None, None, None, None);
        while at < entry.len() {
            match read_varint(entry, &mut at)? {
                10 => set_once(
                    &mut identity,
                    read_bytes(entry, &mut at)?
                        .try_into()
                        .map_err(|_| PartyWireError::Malformed)?,
                )?,
                16 => set_once(&mut generation, read_varint(entry, &mut at)?)?,
                24 => set_once(
                    &mut shield,
                    u8::try_from(read_varint(entry, &mut at)?)
                        .map_err(|_| PartyWireError::Malformed)?,
                )?,
                32 => set_once(
                    &mut skull,
                    u8::try_from(read_varint(entry, &mut at)?)
                        .map_err(|_| PartyWireError::Malformed)?,
                )?,
                _ => return Err(PartyWireError::Malformed),
            }
        }
        icons.push(PartyIcon {
            entity: EntityRef {
                identity: identity.ok_or(PartyWireError::Malformed)?,
                generation: generation.ok_or(PartyWireError::Malformed)?,
            },
            shield: shield.ok_or(PartyWireError::Malformed)?,
            skull: skull.ok_or(PartyWireError::Malformed)?,
        });
    }
    encode_party_icons(&icons)?;
    Ok(icons)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn party_skull_codec_bounds_and_counterfactual() -> Result<(), PartyWireError> {
        let icon = PartyIcon {
            entity: EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            shield: 8,
            skull: 2,
        };
        let bytes = encode_party_icons(&[icon])?;
        assert_eq!(decode_party_icons(&bytes)?, [icon]);
        assert!(encode_party_icons(&[icon, icon]).is_err());
        assert!(decode_party_icons(&bytes[..bytes.len() - 1]).is_err());
        assert!(encode_party_icons(&[PartyIcon { shield: 12, ..icon }]).is_err());
        Ok(())
    }
}
