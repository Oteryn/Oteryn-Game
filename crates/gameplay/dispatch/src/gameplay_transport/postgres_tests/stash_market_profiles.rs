//! Read saved stash balances using the published native Item provider.
use super::death_ml_loss_composed_fixtures::production_generation_with_pin;
use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use crate::domain::CharacterId;
use crate::foundation::WorldId;

#[test]
fn stash_legacy_moonlight_crystals_remain_eligible() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "stash market profile scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "stash_market_profiles", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let character = CharacterId::from_bytes(id(41)).map_err(debug)?;
        let (controller, _) = production_generation_with_pin(world, "content-1")?;
        let native = controller
            .active()
            .and_then(|generation| generation.native_gameplay())
            .ok_or("native pin")?;
        let crystal = "oteryn:item.tibia.i22083";
        let container = "oteryn:item.tibia.i21292";
        // Load saved rows independently of what the current provider can stow.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for key in [crystal, container] {
            sqlx::query("INSERT INTO game_character_stash_balances(character_id,world_id,definition_key,definition_revision,quantity,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'definition-r1',5,game_character_uuid_v7())")
                .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(key)
                .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        let (rows, more) = h.root.read_stash_page(&authority, character, 0, native).await.map_err(debug)?;
        assert!(!more);
        assert_eq!(
            rows.iter().map(|row| (row.definition_key.as_str(), row.quantity, row.migration_required)).collect::<Vec<_>>(),
            [(container, 5, true), (crystal, 5, false)],
        );
        Ok(())
    })
}
