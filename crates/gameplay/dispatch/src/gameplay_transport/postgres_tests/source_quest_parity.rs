use super::{TestResult, quest_catalogue_chests_fixtures as chests};
use oteryn_world_bundle_compiler::{b3, bundle};

#[test]
fn vampire_hunter_storkus_requires_inquisition_and_grants_six_ranks() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-vampire-hunter")
}

#[test]
fn vampire_hunter_diblis_death_entitles_killer_to_one_token() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-vampire-hunter-loot")
}

#[test]
fn restored_quest_chests_use_committed_selectors_and_mint_the_exact_rewards() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/world/placements");
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("index.json"))?)?;
    let mut selected = Vec::new();
    for chest in chests::cases()? {
        if !matches!(
            chest.unique,
            6039..=6044 | 6071 | 6160 | 6258 | 6270..=6271 | 6273..=6274 | 6278
        ) {
            continue;
        }
        let (x, y, floor) = chest.position;
        let path = format!("region-z{:02}-x{:03}-y{:03}.b3", -floor, x / 256, y / 256);
        let mut budget = bundle::BUNDLE_BUDGET;
        let region = b3::decode_region(
            &std::fs::read(directory.join(path))?,
            bundle::TILE_LIMITS,
            &mut budget,
        )?;
        let actual: Vec<_> = region
            .sectors
            .into_iter()
            .flat_map(|(_, _, tiles)| tiles)
            .filter(|tile| (tile.x, tile.y) == (x, y))
            .flat_map(|tile| tile.items)
            .filter(|item| {
                item.depth == 0
                    && index["palette"][item.palette as usize]["source_item_id"] == chest.appearance
            })
            .map(|item| item.attrs.unique)
            .collect();
        assert_eq!(
            actual,
            [Some(chest.unique)],
            "source at {:?}",
            chest.position
        );
        selected.push(chest);
    }
    assert_eq!(selected.len(), 14);
    chests::execute(selected)
}
