//! Closed, hash-qualified M52 map facts. No source map bytes or portable items.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

const CAPTURE: &[u8] =
    include_bytes!("../../../../../imports/crystalserver/grimvale-full-moon-maps.json");
const SHA256: [u8; 32] = [
    0xf7, 0x19, 0x6f, 0xe4, 0x79, 0x6c, 0x68, 0x45, 0xfd, 0x3a, 0xbd, 0xb0, 0x6e, 0xc8, 0xb4, 0x85,
    0xd3, 0x6d, 0x89, 0x53, 0xe2, 0xa8, 0x2d, 0xe6, 0xeb, 0x90, 0xc2, 0xbe, 0x29, 0x40, 0x97, 0xa5,
];

/// A source cell: x, y, source z, source tile flags, ordered source item IDs.
#[derive(Debug, Deserialize)]
pub struct FeroxaSourceTile(pub u16, pub u16, pub u8, pub u16, pub Vec<u16>);

#[derive(Debug, Deserialize)]
pub struct FeroxaSourcePhase {
    pub phase: u8,
    pub after_ms: u64,
    pub tiles: Vec<FeroxaSourceTile>,
    /// x, y, source z, stack ordinal, destination x, y, source z.
    pub teleports: Vec<[u16; 7]>,
}

/// Item-loader facts from the pinned official appearances and source XML.
#[derive(Debug, Deserialize)]
pub struct FeroxaSourceItem {
    pub server_item_id: u16,
    pub ground: bool,
    pub ground_speed: u16,
    pub block_solid: bool,
    pub block_projectile: bool,
    pub block_pathfind: bool,
    pub movable: bool,
    pub pickupable: bool,
    pub container: bool,
    pub floor_change: bool,
    pub has_height: bool,
    pub magic_field: bool,
}

#[derive(Debug, Deserialize)]
pub struct FeroxaSourceMaps {
    phases: Vec<FeroxaSourcePhase>,
    palette: Vec<FeroxaSourceItem>,
}

impl FeroxaSourceMaps {
    /// Immutable admission is cached; an invalid capture never falls back to base scenery.
    pub fn embedded() -> Option<&'static Self> {
        static MAPS: OnceLock<Option<FeroxaSourceMaps>> = OnceLock::new();
        MAPS.get_or_init(|| Self::parse(CAPTURE)).as_ref()
    }

    fn parse(raw: &[u8]) -> Option<Self> {
        if raw.len() > 256 * 1024 || <[u8; 32]>::from(Sha256::digest(raw)) != SHA256 {
            return None;
        }
        serde_json::from_slice(raw).ok()
    }

    /// Select directly from the original occurrence clock, including recovery past both timers.
    pub fn due(&self, elapsed_ms: u64) -> Option<&FeroxaSourcePhase> {
        self.phases
            .iter()
            .rev()
            .find(|phase| phase.after_ms <= elapsed_ms)
    }

    pub const fn capture_digest(&self) -> [u8; 32] {
        SHA256
    }

    /// Native collision/combat facts use the same admitted source stack as rendering.
    pub fn spell_tile(&self, tile: &FeroxaSourceTile) -> Option<super::QualifiedSpellTile> {
        let mut ground = None;
        let mut top = Vec::new();
        let mut flags = super::NativeSpellTileFlags {
            block_solid: false,
            block_projectile: false,
            immovable_block_solid: false,
            immovable_block_item: false,
            immovable_nonfield_block_item: false,
            floor_change: false,
            protection_zone: tile.3 & 1 != 0,
        };
        for id in &tile.4 {
            let p = self.item(*id)?;
            if p.ground {
                if ground.replace((u32::from(*id), p.ground_speed)).is_some() {
                    return None;
                }
            } else {
                top.push(u32::from(*id));
            }
            flags.block_solid |= p.block_solid;
            flags.block_projectile |= p.block_projectile;
            flags.immovable_block_solid |= p.block_solid && !p.movable;
            flags.immovable_block_item |= p.block_pathfind && !p.movable;
            flags.immovable_nonfield_block_item |= p.block_pathfind && !p.movable && !p.magic_field;
            flags.floor_change |= p.floor_change;
        }
        // OTBM no-PvP / PvP-zone bits, distinct from protection/no-logout.
        Some(super::QualifiedSpellTile::from_bundle(
            ground,
            top,
            flags,
            Some((tile.3 & 4 != 0, tile.3 & 16 != 0)),
        ))
    }

    pub fn item(&self, source_id: u16) -> Option<&FeroxaSourceItem> {
        self.palette
            .binary_search_by_key(&source_id, |p| p.server_item_id)
            .ok()
            .and_then(|index| self.palette.get(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qn5_feroxa_source_phase_clock_has_exact_boundaries_and_recovers_latest()
    -> Result<(), Box<dyn std::error::Error>> {
        let maps = FeroxaSourceMaps::embedded().ok_or("qualified source facts")?;
        assert!(maps.due(1_799_999).is_none());
        let middle = maps.due(1_800_000).ok_or("30 minutes")?;
        assert_eq!(middle.phase, 1);
        assert_eq!(maps.due(3_599_999).ok_or("before final")?.phase, 1);
        let final_map = maps.due(3_600_000).ok_or("60 minutes")?;
        assert_eq!(final_map.phase, 2);
        assert_eq!(maps.due(u64::MAX).ok_or("recovered")?.phase, 2);
        for phase in [middle, final_map] {
            assert_eq!(phase.tiles.len(), 1225);
            assert_eq!(phase.teleports, [[33430, 31537, 11, 1, 33457, 31531, 11]]);
            for tile in &phase.tiles {
                assert!((33413..=33447).contains(&tile.0));
                assert!((31520..=31554).contains(&tile.1));
                assert_eq!(tile.2, 11);
                assert!(matches!(tile.3, 0 | 8));
                let spell = maps.spell_tile(tile).ok_or("qualified native tile")?;
                assert!(!spell.flags().floor_change);
                assert!(!spell.flags().protection_zone);
                for id in &tile.4 {
                    let item = maps.item(*id).ok_or("source palette")?;
                    assert!(!item.pickupable && !item.container && !item.floor_change);
                }
            }
        }
        assert_eq!(maps.palette.len(), 158);
        let mut altered = CAPTURE.to_vec();
        altered[20] ^= 1;
        assert!(FeroxaSourceMaps::parse(&altered).is_none());
        assert!(FeroxaSourceMaps::parse(&[]).is_none());
        Ok(())
    }
}
