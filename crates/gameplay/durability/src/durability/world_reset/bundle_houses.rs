//! House placement evidence decoded from a complete, digest-pinned World Bundle.
use oteryn_world_bundle::{Error, bundle};

pub(super) type HouseTile = (i8, u16, u16, u32);

/// The tile order is the validated sector order and each sector's tile order.
/// Epoch layers cannot place Houses or intersect a base House tile.
pub struct BundleHouseCatalogue {
    pub(super) digest: [u8; 32],
    pub(super) map_revision: String,
    tiles: Vec<HouseTile>,
}

impl BundleHouseCatalogue {
    pub fn decode(bytes: &[u8], expected_digest: [u8; 32]) -> Result<Self, Error> {
        if !bytes.ends_with(&expected_digest) {
            return Err(Error::Format("World reset bundle digest mismatch".into()));
        }
        let mut tiles = Vec::new();
        let visited = bundle::visit(bytes, |_, sector| {
            tiles.extend(
                sector.tiles.into_iter().filter(|tile| tile.house != 0)
                    .map(|tile| (sector.floor, tile.x, tile.y, tile.house)),
            );
            Ok(())
        })?;
        let digest_hex: String = visited.digest.iter().map(|byte| format!("{byte:02x}")).collect();
        Ok(Self {
            digest: visited.digest,
            map_revision: format!("sha256:{digest_hex}"),
            tiles,
        })
    }

    pub fn tiles(&self) -> &[(i8, u16, u16, u32)] {
        &self.tiles
    }
}
