//! Pinned Crystal creatures_definitions.hpp: native categories and icon IDs.
use super::*;
pub const CAPABILITY_NATIVE_CREATURE_ICONS_V1: u32 = 371;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCreatureIcon {
    pub category: u8,
    pub id: u8,
    pub count: u16,
}
impl NativeCreatureIcon {
    pub fn validate(self) -> Result<(), WorldSpatialError> {
        if matches!(
            (self.category, self.id, self.count),
            (0, 2, 0..=111)
                | (0, 17, 1..=100)
                | (0, 22, 1..=u16::MAX)
                | (1, 6, 1 | 5 | 15)
                | (1, 7, 0)
        ) {
            Ok(())
        } else {
            Err(WorldSpatialError::Malformed)
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeCreatureIcons {
    entries: [Option<NativeCreatureIcon>; 3],
}
impl NativeCreatureIcons {
    pub fn new(entries: &[NativeCreatureIcon]) -> Result<Self, WorldSpatialError> {
        if entries.len() > 3 {
            return Err(WorldSpatialError::LimitExceeded);
        }
        let mut result = Self::default();
        for (slot, entry) in result.entries.iter_mut().zip(entries) {
            entry.validate()?;
            *slot = Some(*entry);
        }
        Ok(result)
    }
    pub fn iter(&self) -> impl Iterator<Item = NativeCreatureIcon> + '_ {
        self.entries.iter().flatten().copied()
    }
    pub(super) fn push(&mut self, entry: NativeCreatureIcon) -> Result<(), WorldSpatialError> {
        entry.validate()?;
        let slot = self
            .entries
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(WorldSpatialError::LimitExceeded)?;
        *slot = Some(entry);
        Ok(())
    }
}
pub(super) fn encode(icon: NativeCreatureIcon) -> Vec<u8> {
    let mut result = Vec::new();
    for (field, value) in [
        (1, u64::from(icon.category)),
        (2, u64::from(icon.id)),
        (3, u64::from(icon.count)),
    ] {
        push_tag(&mut result, field, 0);
        push_varint(&mut result, value);
    }
    result
}
pub(super) fn decode(input: &[u8]) -> Result<NativeCreatureIcon, WorldSpatialError> {
    let (mut category, mut id, mut count) = (None, None, None);
    let mut cursor = 0;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            8 => once(&mut category, varint_u32(input, &mut cursor)?)?,
            16 => once(&mut id, varint_u32(input, &mut cursor)?)?,
            24 => once(&mut count, varint_u32(input, &mut cursor)?)?,
            _ => return Err(WorldSpatialError::Malformed),
        }
    }
    let result = NativeCreatureIcon {
        category: u8::try_from(category.ok_or(WorldSpatialError::Malformed)?)
            .map_err(|_| WorldSpatialError::Malformed)?,
        id: u8::try_from(id.ok_or(WorldSpatialError::Malformed)?)
            .map_err(|_| WorldSpatialError::Malformed)?,
        count: u16::try_from(count.ok_or(WorldSpatialError::Malformed)?)
            .map_err(|_| WorldSpatialError::Malformed)?,
    };
    result.validate()?;
    Ok(result)
}
