//! D1215: durability-local reset inputs. No Content or transport imports.
//!
//! Implementations belong to the admitted House source and I's genuine drain proof.
//! A consumer must recheck the World/current generation under its retained TX locks;
//! these traits do not replace the locked job, census or node-incarnation checks.

use super::runtime_scope_assignment::NodeRegistrationFact;
use crate::foundation::WorldId;
use oteryn_game_server::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};

pub(crate) struct HouseResetFact {
    pub house_key: String,
    /// Canonical qualified House definition digest from its native source owner.
    pub definition_digest: [u8; 32],
    /// Exact native House-position bytes from C/#2163, never a copied encoder.
    /// The consuming preflight must enforce D0025's 1..=128-byte bound.
    pub spatial_position: Vec<u8>,
}

pub(crate) struct WorldResetSourceRefused;

#[derive(Clone, Copy)]
pub(crate) enum CatalogueSide {
    Active,
    Target,
}

pub(crate) trait WorldResetHouseSource {
    fn reset_houses(
        &self,
        world: WorldId,
        side: CatalogueSide,
    ) -> Result<Vec<HouseResetFact>, WorldResetSourceRefused>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DrainedChannelBinding {
    pub(crate) scope: RuntimeScopeRefV1,
    pub(crate) generation: u64,
}

/// Data borrowed from I's sealed proof. This struct alone is never current authority.
pub(crate) struct WorldDrainBinding {
    pub(crate) world: WorldId,
    pub(crate) node: NodeRegistrationFact,
    pub(crate) channels: Vec<DrainedChannelBinding>,
    pub(crate) cohort_digest: [u8; 32],
}

pub(crate) trait WorldDrainWitness {
    fn drained(&self) -> &WorldDrainBinding;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorldDrainBindingRefused;

/// The single D1233 canonical digest implementation, called by I's proof producer
/// and G's locked consumer. This is an internal data binding, not an RNG/wire domain
/// or authority constructor. Inputs must already be strictly sorted by ChannelId bytes.
/// Encoding: fixed domain+version, World16, Node16, registration u64BE, count u64BE,
/// then (Channel16,generation u64BE) per entry. Every Channel is in this World.
pub(crate) fn world_drain_cohort_digest(
    world: WorldId,
    node: NodeRegistrationFact,
    channels: &[DrainedChannelBinding],
) -> Result<[u8; 32], WorldDrainBindingRefused> {
    if node.registration_revision() == 0 {
        return Err(WorldDrainBindingRefused);
    }
    let mut hash = Sha256::new();
    hash.update(b"OTERYN_WORLD_RESET_DRAIN_BINDING\0\x01");
    hash.update(world.as_bytes());
    hash.update(node.node_id().as_bytes());
    hash.update(node.registration_revision().to_be_bytes());
    hash.update(
        u64::try_from(channels.len())
            .map_err(|_| WorldDrainBindingRefused)?
            .to_be_bytes(),
    );
    let mut previous = None;
    for channel in channels {
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = channel.scope
        else {
            return Err(WorldDrainBindingRefused);
        };
        if world_id != world
            || channel.generation == 0
            || previous.is_some_and(|old| old >= *channel_id.as_bytes())
        {
            return Err(WorldDrainBindingRefused);
        }
        previous = Some(*channel_id.as_bytes());
        hash.update(channel_id.as_bytes());
        hash.update(channel.generation.to_be_bytes());
    }
    Ok(hash.finalize().into())
}
