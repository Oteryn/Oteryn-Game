//! Secret Library compatibility regression through the native quest journey.
use super::TestResult;

#[test]
fn qcx_the_secret_library_gail_reports_her_daughters_fate() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::questpc8_secret_library_fixtures::run_library_journey()?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx_the_secret_library_gail_reports_her_daughters_fate duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn qcx_the_secret_library_hawser_claim() -> TestResult {
    use super::fixture::{debug, id};
    use crate::map::overlay::TilePos;

    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("high-and-dry-hawser", || {
        let claim = super::quest_catalogue_chests_fixtures::CatalogueChest {
            claim: "oteryn:reward-claim.quest.u11_80.the_secret_library.small_islands.hawser"
                .into(),
            position: (32025, 32469, -7),
            unique: 6330,
            servable: true,
            appearance: 5951,
            root: "oteryn:item.tibia.i28707".into(),
            quantity: 1,
            contents: Vec::new(),
            texts: Vec::new(),
            key_number: None,
            achievement: false,
            charges: Vec::new(),
            fluids: Vec::new(),
        };
        let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let activation =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(world_id, "content-1")?;
        let native = activation
            .0
            .active()
            .and_then(|generation| generation.native_gameplay())
            .ok_or("native generation")?;
        let minimum = TilePos {
            x: 32025,
            y: 32469,
            floor: -7,
        };
        let start = TilePos {
            x: 32026,
            ..minimum
        };
        let world = super::source_world_crop::world(minimum, start, start, native)?;
        let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
            world.base().as_ref(),
            &|_| true,
        )?;
        assert!(
            report
                .served
                .iter()
                .any(|row| row.key.as_str() == claim.claim),
            "shipped map must bind the High and Dry Hawser"
        );
        super::quest_container_chests_fixtures::catalogue_scenario(&claim, &activation)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx_the_secret_library_hawser_claim duration={:?}",
        started.elapsed()
    );
    Ok(())
}
