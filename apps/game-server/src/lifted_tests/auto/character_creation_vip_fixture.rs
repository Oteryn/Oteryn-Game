//! Retained VIP fixture creation choices survive the production recovery verifier.
use crate::bestiary_postgres_harness::{Harness, TestResult, WORLD, debug, id};

#[path = "character_creation_vip_fixture/fixture.rs"]
mod fixture;
use fixture::fixture_character_world;

#[test]
fn vip_fixture_bootstrap_v4_binding_survives_production_recovery() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            (|| -> TestResult {
                crate::bestiary_postgres_harness::runtime()?.block_on(async {
                    let h = Harness::create(admin, "vip_creation_v4", true).await?;
                    let result = async {
                        fixture_character_world(&h, 60, 70, WORLD).await?;
                        let seal = h.recovery.seal_current().map_err(debug)?;
                        let authority = h
                            .root
                            .open_character_authority(&seal)
                            .await
                            .map_err(debug)?;
                        let character =
                            crate::domain::CharacterId::from_bytes(id(60)).map_err(debug)?;
                        let record = h
                            .root
                            .read_current_character(&authority, character)
                            .await
                            .map_err(debug)?;
                        assert_eq!(record.revision.get(), 1);
                        let build = h
                            .root
                            .read_character_build_state(&authority, character)
                            .await
                            .map_err(debug)?;
                        assert_eq!(
                            build,
                            crate::durability::character_build::DurableBuildState::new(
                                "knight",
                                (0, 0),
                                [(10, 0); 7]
                            )
                            .map_err(debug)?
                        );
                        assert_eq!(
                            h.root.read_character_sex(&authority, character).await?,
                            crate::domain::appearance::Sex::Male
                        );
                        eprintln!("VIP_CREATION_V4_PG_EXECUTED");
                        Ok(())
                    }
                    .await;
                    h.cleanup().await?;
                    result
                })
            })()
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "VIP creation fixture worker panicked")?
        .map_err(Into::into)
}
