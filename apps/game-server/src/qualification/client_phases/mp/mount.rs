//! Multi-player scenarios, mounted once from the world journey.
use super::*;

mod bots;
mod diagnostics;
mod mp01_party_pvp;
mod mp07_chat;
mod mp06_guild_war;

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
    verdict::indexed("MP07.full", mp07_chat::run(core, packets, world, towns)).await;
    verdict::indexed(
        "MP06.full",
        mp06_guild_war::run(core, packets, world, towns),
    )
    .await;
}
