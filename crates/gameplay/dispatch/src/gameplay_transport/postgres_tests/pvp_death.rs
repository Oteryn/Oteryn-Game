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
        request.public_cause = Some(
            crate::durability::character_death::PublicDeathCause::Player {
                character_id: id(60),
            },
        );
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
        let identity:(String,String,bool)=sqlx::query_as("SELECT k.kind,k.character_id::text,c.pvp FROM game_public_death_killers k JOIN game_public_death_classifications c USING(death_occurrence_id)").fetch_one(&harness.pool).await?;
        assert_eq!(
            identity,
            (
                "player".into(),
                sqlx::query_scalar::<_, String>("SELECT encode($1,'hex')::uuid::text")
                    .bind(id(60).as_slice())
                    .fetch_one(&harness.pool)
                    .await?,
                true
            )
        );

        let killer_key: String =
            sqlx::query_scalar("SELECT killer_key FROM game_public_death_classifications")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(
            killer_key, "player",
            "admin primary cause never exports CharacterId"
        );
        // Privacy is separate from private historical identity. A player-owned
        // summon is captured as this same player cohort, never as creature bypass.
        let mut hidden = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *hidden)
            .await?;
        sqlx::query(
            "UPDATE game_character_roots SET lifecycle=2 WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(60).as_slice())
        .execute(&mut *hidden)
        .await?;
        hidden.commit().await?;
        sqlx::query("SELECT game_public_v2_deaths(encode($1,'hex')::uuid)")
            .bind(id(41).as_slice())
            .execute(&harness.pool)
            .await?;
        let hidden:bool=sqlx::query_scalar("SELECT payload->0->'killers'='[]'::jsonb FROM game_public_v2_heads WHERE family='character_deaths' AND subject=encode($1,'hex')::uuid::text").bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
        assert!(
            hidden,
            "hidden player identity and name cannot escape via a summon"
        );
        let blessings: Vec<String> = sqlx::query_scalar(
            "SELECT blessing_key FROM game_character_blessings \
             WHERE character_id = encode($1,'hex')::uuid ORDER BY blessing_key",
        )
        .bind(id(41).as_slice())
        .fetch_all(&harness.pool)
        .await?;
        assert_eq!(blessings, ["oteryn:blessing.spiritual_shielding"]);
        eprintln!(
            "PVP_POSTGRES_EVIDENCE: dispatch death committed on PostgreSQL 18.6 with migration 0142"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
