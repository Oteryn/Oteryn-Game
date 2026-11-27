//! Legacy reward claims bind the actual untagged B3 source cells.
use super::fixture::{debug, id};
use super::{TestResult, quest_catalogue_chests_fixtures::cases};
use crate::map::overlay::TilePos;

const CROSSBOW: &str = "oteryn:reward-claim.quest.u7_8.hunter_outfits.elane_crossbow";
const INUKAYA: &str = "oteryn:reward-claim.quest.u8_0.shardsof_ancient_winters.inukaya";

fn untagged_claim(key: &'static str, source_uid: u16, start: TilePos) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("untagged-reward-chest", move || {
        let mut selected = cases()?
            .into_iter()
            .filter(|chest| chest.claim == key)
            .collect::<Vec<_>>();
        assert_eq!(selected.len(), 1);
        let mut chest = selected.pop().ok_or("native source claim")?;
        assert_eq!(chest.unique, source_uid);
        let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let activation = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            world_id,
            "content-1",
        )?;
        let native = activation
            .0
            .active()
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        let (x, y, floor) = chest.position;
        let world = super::questpc8_native_slice::world(
            native,
            &[(x - 2, y - 2, x + 2, y + 2, floor)],
            &[],
            start,
        )?;
        let tile = world.base().tile(x, y, floor).ok_or("real source tile")?;
        let ordinal = super::qc_pinned_world::ordinal(tile, chest.appearance, None)?;
        assert!(tile.unique(ordinal).is_none());
        let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
            world.base().as_ref(),
            &|_| true,
        )?;
        let claim = report
            .served
            .iter()
            .find(|claim| claim.key.as_str() == key)
            .ok_or_else(|| format!("live untagged claim {key} was not bound"))?;
        assert_eq!(claim.placements.len(), 1);
        assert_eq!(
            Some(claim.placements[0].bundle_placement_key),
            oteryn_world_bundle::bundle::placement_key(floor, x, y, ordinal)
        );
        // Exercise the same untagged selector through USE, replay and durable MINT.
        chest.unique = 0;
        super::quest_container_chests_fixtures::catalogue_scenario(&chest, &activation)
    })
}

#[test]
fn hunter_crossbow_untagged_cache_mints_once() -> TestResult {
    untagged_claim(
        CROSSBOW,
        6191,
        TilePos {
            x: 32369,
            y: 32797,
            floor: -10,
        },
    )
}

#[test]
fn inukaya_untagged_igloo_barrel_mints_fur_bag_once() -> TestResult {
    untagged_claim(
        INUKAYA,
        6215,
        TilePos {
            x: 32484,
            y: 31109,
            floor: -8,
        },
    )
}
