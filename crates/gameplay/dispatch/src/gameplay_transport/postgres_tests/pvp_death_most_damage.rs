//! Capability 488: the fenced death owner captures the most-damage dealer from the PvP harm
//! receipts, not the final blow.
use super::fixture::{Harness, debug, fence, id, pvp_source_death_fixture, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::CharacterId;
use crate::durability::pvp_death_consequences::{PvpContributorKind, PvpDeathContributor};
use crate::gameplay_transport::death_ml_loss_postgres::death as death_request;

#[test]
fn pvp_death_captures_the_largest_harm_total_as_the_most_damage_dealer() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "pvp_death_most_damage", true, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        // Two attackers of the session Character: 60 ("Pvpci") lands the final blow,
        // 61 ("Pvpcj") dealt more damage.
        pvp_source_death_fixture::seed_root(&harness.pool, 41, 60, 42).await?;
        pvp_source_death_fixture::seed_root(&harness.pool, 41, 61, 42).await?;
        sqlx::query("INSERT INTO game_character_pvp_state (character_id, world_id) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&harness.pool).await?;
        for attacker in [60, 61] {
            pvp_source_death_fixture::seed_ledger(
                &harness.pool,
                41,
                attacker,
                true,
                Some(1),
                "0 seconds",
            )
            .await?;
        }
        // Harm receipts are fixture setup only; 61 deals 40 + 30, 60 deals 50.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        for (ordinal, attacker, damage) in [(0_i32, 60, 50_i32), (1, 61, 40), (2, 61, 30)] {
            sqlx::query("INSERT INTO game_character_pvp_harm(game_session_id,command_id,ordinal,victim,attacker,binding,damage,unjustified) \
                VALUES(encode($3,'hex')::uuid,1,$5,encode($1,'hex')::uuid,encode($2,'hex')::uuid,$4,$6,true)")
                .bind(id(41).as_slice()).bind(id(attacker).as_slice()).bind(id(77).as_slice())
                .bind([7_u8; 32].as_slice()).bind(ordinal).bind(damage).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        let mut request = death_request(90)?;
        for attacker in [60, 61] {
            request.pvp_contributors.push(PvpDeathContributor {
                character_id: CharacterId::from_bytes(id(attacker)).map_err(debug)?,
                kind: PvpContributorKind::Damage,
                unjustified: true,
            });
        }
        request.public_cause = Some(
            crate::durability::character_death::PublicDeathCause::Player {
                character_id: id(60),
            },
        );
        let mut current = fence(1)?;
        current.character_lease_generation = 2;
        owner
            .settle_source_player_death(current, request, None, None)
            .await
            .map_err(debug)?;
        let captured: (i16, String, Option<i16>, Option<String>) = sqlx::query_as(
            "SELECT k.ordinal,k.name,c.most_damage_ordinal,m.name FROM game_public_death_classifications c \
             JOIN game_public_death_killers k ON k.death_occurrence_id=c.death_occurrence_id AND k.ordinal=0 \
             LEFT JOIN game_public_death_killers m ON m.death_occurrence_id=c.death_occurrence_id AND m.ordinal=c.most_damage_ordinal",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(
            captured,
            (0, "Pvpci".into(), Some(1), Some("Pvpcj".into())),
            "the final blow stays ordinal 0 and the larger harm total is the most-damage dealer"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
