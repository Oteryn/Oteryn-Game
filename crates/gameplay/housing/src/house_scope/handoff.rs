//! Entry tile selection for a house handoff (HOUSE-RUNTIME-0 §4.2).

use super::{HOUSE_SCOPE_CHARACTERS_MAX, HouseEntryRefusal};

/// The first door-adjacent candidate tile neither occupied nor reserved, or `NoRoom` when
/// the house is full or every candidate is taken. Tiles are HouseInterior
/// `spatial_position` encodings.
pub fn select_entry_tile(
    candidates: &[Vec<u8>],
    occupied: &[Vec<u8>],
    reserved: &[Vec<u8>],
    inside_count: u64,
) -> Result<Vec<u8>, HouseEntryRefusal> {
    if inside_count >= HOUSE_SCOPE_CHARACTERS_MAX {
        return Err(HouseEntryRefusal::NoRoom);
    }
    candidates
        .iter()
        .find(|tile| !occupied.contains(tile) && !reserved.contains(tile))
        .cloned()
        .ok_or(HouseEntryRefusal::NoRoom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_the_first_free_tile_or_refuses() {
        let tiles = vec![vec![1], vec![2], vec![3]];
        assert_eq!(
            select_entry_tile(&tiles, &[vec![1]], &[vec![2]], 0),
            Ok(vec![3])
        );
        assert_eq!(
            select_entry_tile(&tiles, &[vec![1], vec![3]], &[vec![2]], 2),
            Err(HouseEntryRefusal::NoRoom)
        );
        assert_eq!(
            select_entry_tile(&tiles, &[], &[], HOUSE_SCOPE_CHARACTERS_MAX),
            Err(HouseEntryRefusal::NoRoom)
        );
    }
}
