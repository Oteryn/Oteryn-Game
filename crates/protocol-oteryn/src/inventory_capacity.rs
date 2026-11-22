//! INVCAP-1: the own character's carrying capacity and free capacity, as Tibia Global shows
//! `Cap` in the inventory panel. Sent only to a session that selected capability 508, as the
//! sole optional field 7 of a domain-1 type-2 visibility delta (`inventory-capacity-v1.md`).
use crate::charm_wire::{push_varint_field, read_varint, set_once};
use crate::world_spatial::WorldSpatialError;

pub const CAPABILITY_INVENTORY_CAPACITY_V1: u32 = 508;
/// Two tagged varints of at most five bytes each.
pub const MAX_INVENTORY_CAPACITY_BYTES: usize = 12;

/// Both values are in the server's item-weight unit, centi-ounces (1/100 oz), like Tibia's.
/// `free` is the capacity left after the carried weight, never more than `capacity`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InventoryCapacity {
    pub capacity: u32,
    pub free: u32,
}

impl InventoryCapacity {
    /// From the derived capacity and the carried weight; `None` past the wire bound.
    #[must_use]
    pub fn from_carried(capacity: u64, carried: u64) -> Option<Self> {
        Some(Self {
            capacity: u32::try_from(capacity).ok()?,
            free: u32::try_from(capacity.saturating_sub(carried)).ok()?,
        })
    }

    /// Whole ounces of free capacity, as the client shows them.
    #[must_use]
    pub const fn free_oz(self) -> u32 {
        self.free / 100
    }

    pub fn encode(self) -> Result<Vec<u8>, WorldSpatialError> {
        if self.free > self.capacity {
            return Err(WorldSpatialError::Malformed);
        }
        let mut out = Vec::with_capacity(MAX_INVENTORY_CAPACITY_BYTES);
        push_varint_field(&mut out, 1, u64::from(self.capacity));
        push_varint_field(&mut out, 2, u64::from(self.free));
        Ok(out)
    }

    pub fn decode(input: &[u8]) -> Result<Self, WorldSpatialError> {
        if input.len() > MAX_INVENTORY_CAPACITY_BYTES {
            return Err(WorldSpatialError::LimitExceeded);
        }
        let malformed = |_| WorldSpatialError::Malformed;
        let (mut cursor, mut capacity, mut free) = (0, None, None);
        while cursor < input.len() {
            let slot = match read_varint(input, &mut cursor).map_err(malformed)? {
                8 => &mut capacity,
                16 => &mut free,
                _ => return Err(WorldSpatialError::Malformed),
            };
            let value = u32::try_from(read_varint(input, &mut cursor).map_err(malformed)?)
                .map_err(|_| WorldSpatialError::Malformed)?;
            set_once(slot, value).map_err(malformed)?;
        }
        let value = Self {
            capacity: capacity.ok_or(WorldSpatialError::Malformed)?,
            free: free.ok_or(WorldSpatialError::Malformed)?,
        };
        if value.encode()? != input {
            return Err(WorldSpatialError::Malformed);
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_codec_is_bounded_canonical_and_free_never_exceeds_capacity()
    -> Result<(), WorldSpatialError> {
        let value =
            InventoryCapacity::from_carried(60_000, 1_250).ok_or(WorldSpatialError::Malformed)?;
        assert_eq!(
            value,
            InventoryCapacity {
                capacity: 60_000,
                free: 58_750
            }
        );
        assert_eq!(value.free_oz(), 587);
        let bytes = value.encode()?;
        assert_eq!(InventoryCapacity::decode(&bytes)?, value);
        let max = InventoryCapacity {
            capacity: u32::MAX,
            free: u32::MAX,
        };
        assert_eq!(max.encode()?.len(), MAX_INVENTORY_CAPACITY_BYTES);
        // Overloaded: free capacity is zero, never negative.
        assert_eq!(
            InventoryCapacity::from_carried(100, 250).map(|v| v.free),
            Some(0)
        );
        assert!(InventoryCapacity::from_carried(u64::from(u32::MAX) + 1, 0).is_none());
        assert!(
            InventoryCapacity {
                capacity: 1,
                free: 2
            }
            .encode()
            .is_err()
        );
        assert!(InventoryCapacity::decode(&[8, 1, 16, 2]).is_err());
        assert!(InventoryCapacity::decode(&[8, 1]).is_err());
        assert!(InventoryCapacity::decode(&[8, 1, 8, 1, 16, 1]).is_err());
        assert!(InventoryCapacity::decode(&[16, 1, 8, 1]).is_err());
        assert!(InventoryCapacity::decode(&[0x18, 1]).is_err());
        assert!(InventoryCapacity::decode(&[0; 13]).is_err());
        Ok(())
    }
}
