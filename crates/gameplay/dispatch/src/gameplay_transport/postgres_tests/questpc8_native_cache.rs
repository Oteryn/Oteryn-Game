//! Native source selector aliases retain the original once-only reward identity.
use super::TestResult;
use super::fixture::{debug, id};

#[test]
fn questpc8_mintwallin_native_cache_without_uid_mints_ruby_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-native-cache", || {
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let activation =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let chest = super::quest_catalogue_chests::CatalogueChest {
            claim: "oteryn:reward-claim.kv.mintwallincyclops".into(),
            position: (32591, 32097, -14),
            unique: 0,
            servable: true,
            appearance: 2472,
            root: "oteryn:item.tibia.i3028".into(),
            quantity: 1,
            contents: vec![],
            texts: vec![],
            key_number: None,
            achievement: false,
            charges: vec![],
            fluids: vec![],
        };
        super::quest_container_chests::catalogue_scenario(&chest, &activation)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_mintwallin_native_cache duration={:?}",
        started.elapsed()
    );
    Ok(())
}
