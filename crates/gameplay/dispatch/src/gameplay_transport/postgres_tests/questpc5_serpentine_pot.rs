//! Native custody facts required by the retained pot-conditioned source route.
use super::TestResult;
#[test]
fn questpc5_serpentine_pot_has_native_custody_admission() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "serpentine_pot",
            false,
            false,
        )
        .await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED serpentine_pot server_version={version}");
        let deps = super::Dependencies::new()?;
        let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let definition = crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i3465".into(),
            revision_ref: "definition-r1".into(),
        };
        crate::gameplay_transport::item_ref_admission::generation_item_facts(native, &definition)
            .map_err(super::fixture::debug)?;
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED serpentine_pot duration={:?}",
            started.elapsed()
        );
        Ok(())
    })
}
