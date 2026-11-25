//! Static Item collision projected by build_crystal_item_collision.py from Crystal 201189f8.

/// The exact bytes used by both the compiler and the map boot owner.
pub const SOURCE_BYTES: &[u8] =
    include_bytes!("../../../content/world/terrain/crystal-item-collision.bin");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemCollision {
    pub solid: bool,
    pub blocks_projectile: bool,
    pub blocks_pathfinding: bool,
    pub door: bool,
}

/// Canonical Crystal ids use their loaded properties or the empty ItemType defaults.
/// Other Item namespaces need their own collision facts, regardless of presentation.
pub fn for_key(key: &str) -> Option<ItemCollision> {
    let id: u16 = key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()?;
    let flags = SOURCE_BYTES.get(usize::from(id)).copied().unwrap_or(0);
    Some(ItemCollision {
        solid: flags & 2 != 0,
        blocks_projectile: flags & 4 != 0,
        blocks_pathfinding: flags & 8 != 0,
        door: flags & 16 != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_crystal_item_uses_item_type_defaults() {
        assert_eq!(
            for_key("oteryn:item.tibia.i65535"),
            Some(ItemCollision {
                solid: false,
                blocks_projectile: false,
                blocks_pathfinding: false,
                door: false,
            })
        );
        assert_eq!(for_key("oteryn:item.custom.i65535"), None);
    }
}
