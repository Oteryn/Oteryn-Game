//! Qualified death input through the production dispatch seam; live attacks stay gated.
use super::fixture::{Harness, debug, fence, id, pvp_source_death_fixture, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::CharacterId;
use crate::durability::pvp_death_consequences::{PvpContributorKind, PvpDeathContributor};
use crate::gameplay_transport::death_ml_loss_postgres::death as death_request;

#[test]
fn death_protection_postgres_projects_exact_committed_consumption_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| death_projection_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "death projection scenario panicked")??;
    Ok(())
}
fn death_projection_scenario() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "death_protection_projection", true, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        dependencies
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(dependencies.actor)
            .map_err(debug)?;
        let owner = dependencies.owner(&harness, &authority);
        dependencies.initialize(&owner).await;
        pvp_source_death_fixture::seed_root(&harness.pool, 41, 60, 42).await?;
        // Compact ledger evidence is a fixture: no attack/classifier is activated.
        sqlx::query("INSERT INTO game_character_pvp_state (character_id, world_id) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&harness.pool).await?;
        pvp_source_death_fixture::seed_ledger(&harness.pool, 41, 60, true, Some(1), "0 seconds")
            .await?;
        let mut request = death_request(90)?;
        // Held-blessing fixture; native purchases are tested through dispatch separately.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_blessings(character_id, blessing_key, provenance) VALUES (encode($1,'hex')::uuid, 'oteryn:blessing.spiritual_shielding', 'fixture'), (encode($1,'hex')::uuid, 'oteryn:blessing.twist_of_fate', 'fixture')")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        request.held_blessings = vec![
            "oteryn:blessing.spiritual_shielding".into(),
            "oteryn:blessing.twist_of_fate".into(),
        ];
        request.pvp_contributors = vec![PvpDeathContributor {
            character_id: CharacterId::from_bytes(id(60)).map_err(debug)?,
            kind: PvpContributorKind::Damage,
            unjustified: true,
        }];
        let mut current = fence(1)?;
        current.character_lease_generation = 2;
        let death = owner
            .settle_source_player_death(current, request, None, None)
            .await
            .map_err(debug)?;
        assert_eq!(
            death.character_id,
            CharacterId::from_bytes(id(41)).map_err(debug)?
        );
        let consequences: (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_character_unjustified_points WHERE points_milli = 1000), \
             (SELECT count(*) FROM game_character_revenge_marks WHERE expires_at = created_at + interval '7 days'), \
             (SELECT count(*) FROM game_character_pvp_ledger), \
             (SELECT count(*) FROM game_character_pvp_state WHERE white_skull_until >= kill_block_until)",
        ).fetch_one(&harness.pool).await?;
        assert_eq!(consequences, (1, 1, 0, 1));
        let blessings: Vec<String> = sqlx::query_scalar(
            "SELECT blessing_key FROM game_character_blessings \
             WHERE character_id = encode($1,'hex')::uuid ORDER BY blessing_key",
        )
        .bind(id(41).as_slice())
        .fetch_all(&harness.pool)
        .await?;
        assert_eq!(blessings, ["oteryn:blessing.spiritual_shielding"]);
        use super::cyclopedia::support::exchange;
        use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
        use crate::gameplay_transport::connection::FirstEntryOutcome;
        use oteryn_protocol_oteryn::death_protection::DeathProtectionReply;
        let mut after_death = current;
        after_death.expected_character_revision = death.committed_character_revision;
        let consumed_respawn = owner
            .root
            .consume_pending_respawn(
                owner.character,
                owner.holder,
                after_death,
                Some(death.occurrence),
            )
            .await
            .map_err(debug)?;
        assert!(
            consumed_respawn.is_some(),
            "latest receipt must survive pending respawn consumption for login"
        );
        let mut admitted = dependencies.admitted;
        admitted.item_fence = Some(current.item_fence());
        admitted.continuity.connection_generation = current.connection_generation.get();
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let offers = [OfferedCapability {
            id: 382,
            requires: &[],
        }];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[382]).ok_or("death capability")?;
        let (status, bytes) = exchange(&owner, &mut admitted, 1, 664, &[]).await?;
        assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
        let projection = DeathProtectionReply::decode(&bytes)
            .ok_or("death protection wire")?
            .0
            .ok_or("retained death")?;
        assert_eq!(projection.occurrence, *death.occurrence.as_bytes());
        assert_eq!(projection.consumed, ["oteryn:blessing.twist_of_fate"]);
        assert_eq!(
            exchange(&owner, &mut admitted, 1, 664, &[]).await?,
            (
                oteryn_protocol_oteryn::CommandStatus::DuplicateReplay,
                bytes
            )
        );
        let mut stale = admitted;
        stale.continuity.connection_generation += 1;
        assert!(owner.death_protection_page(stale).await.is_none());
        eprintln!(
            "NX8_C20c_POSTGRES_EVIDENCE: real PvP death, retained receipt query, router replay, and stale connection refusal executed"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
