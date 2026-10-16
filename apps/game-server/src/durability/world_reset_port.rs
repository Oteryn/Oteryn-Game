//! D1215: durability-local reset inputs. No Content or transport imports.
//!
//! Implementations belong to the admitted House source and I's genuine drain proof.
//! A consumer must recheck the World/current generation under its retained TX locks;
//! these traits do not replace the locked job, census or node-incarnation checks.

use crate::foundation::WorldId;
use oteryn_game_server::foundation::RuntimeScopeRefV1;

pub(crate) struct HouseResetFact {
    pub house_key: String,
    /// Exact native House-position bytes from C/#2163, never a copied encoder.
    /// The consuming preflight must enforce D0025's 1..=128-byte bound.
    pub spatial_position: Vec<u8>,
}

pub(crate) struct WorldResetSourceRefused;

pub(crate) trait WorldResetHouseSource {
    fn reset_houses(&self, world: WorldId) -> Result<Vec<HouseResetFact>, WorldResetSourceRefused>;
}

pub(crate) trait WorldDrainWitness {
    fn drained(&self) -> (RuntimeScopeRefV1, u64);
}
