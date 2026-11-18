//! Multi-player scenarios, mounted once from the world journey.
use super::*;

mod bots;
mod diagnostics;
mod mp01_party_pvp;
mod mp09_party_xp;

pub(super) async fn run(
    core: &SeamClients<'_>,
    packets: &SeamClients<'_>,
    world: &BundleWorld,
    towns: &oteryn_world_bundle::subarea::Table,
) {
    verdict::indexed(
        "MP01.full",
        mp01_party_pvp::run(core, packets, world, towns),
    )
    .await;
    verdict::indexed("MP09.full", mp09_party_xp::run(core, packets, world)).await;
}
