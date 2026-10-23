//! Qualified death input through the production dispatch seam; live attacks stay gated.
use super::fixture::{Harness, debug, fence, id, pvp_source_death_fixture, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::CharacterId;
use crate::durability::pvp_death_consequences::{PvpContributorKind, PvpDeathContributor};
use crate::gameplay_transport::death_ml_loss_postgres::death as death_request;

#[test]
fn pvp_death_dispatch_commits_points_marks_and_skull_state_together() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "pvp_death_dispatch", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        pvp_source_death_fixture::seed_root(&harness.pool, 41, 60, 42).await?;
        // Compact ledger evidence is a fixture: no attack/classifier is activated.
        sqlx::query("INSERT INTO game_character_pvp_state (character_id, world_id) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&harness.pool).await?;
        pvp_source_death_fixture::seed_ledger(&harness.pool, 41, 60, true, Some(1), "0 seconds")
            .await?;
        let mut request = death_request(90)?;
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
        eprintln!(
            "PVP_POSTGRES_EVIDENCE: dispatch death committed on PostgreSQL 18.6 with migration 0142"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
