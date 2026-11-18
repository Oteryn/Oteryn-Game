//! Multi-player scenarios, mounted once from the world journey.
use super::*;

mod bots;
mod diagnostics;
pub(super) mod mp01_party_pvp;
mod mp09_party_xp;
mod observations;

pub(super) async fn run(
    core: &SeamClients<'_>,
    packets: &SeamClients<'_>,
    underlevel: &SeamClients<'_>,
    world: &BundleWorld,
    towns: &oteryn_world_bundle::subarea::Table,
) -> TestResult {
    if E2eMode::from_env()? == E2eMode::Journey {
        for scenario in ["MP01.full", "MP09.full"] {
            evidence(&format!(
                "e2e_group_gap name=mp scenario={scenario} cause=legacy-build-requires-isolated-mode"
            ));
            verdict::indexed(scenario, async {
                Err(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!())).into())
            })
            .await;
        }
        return Ok(());
    }
    let mp01 = verdict::indexed_checked(
        "MP01.full",
        mp01_party_pvp::run(core, packets, world, towns),
    )
    .await;
    let mp09 = verdict::indexed_checked(
        "MP09.full",
        mp09_party_xp::run(core, packets, underlevel, world),
    )
    .await;
    mp01.and(mp09)
}
