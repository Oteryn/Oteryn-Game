//! PVP-1b through the real sequencer and victim death transaction.
use super::*;
use crate::character_progression_postgres_cases::pvp_death_request;
use crate::durability::character_death::{CharacterDeathOutcome, CharacterDeathRequest};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::pvp_death_consequences::{PvpContributorKind, PvpDeathContributor};
use crate::durability::spell_owner_commit::SpellLane;

pub(super) async fn seed_skull_fixture(h: &Harness, seed: u8, duration: &str) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE game_character_pvp_state SET skull = 2, skull_until = clock_timestamp() + interval '{duration}', \
         adventurer_forfeited = TRUE, revision = revision + 1 WHERE character_id = encode($1,'hex')::uuid"
    ))).bind(id(seed).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn ledger(h: &Harness, contributor: u8, kind: i16) -> TestResult {
    // Snapshot persistence/classification is not enabled by this card. Supply compact
    // evidence under the real 0081 World/cap guard, then exercise the real death writer.
    super::pvp_ledger_fixture::insert(
        &h.pool,
        CHARACTER,
        contributor,
        true,
        Some(kind),
        "0 seconds",
    )
    .await?;
    Ok(())
}

fn request(
    tag: u8,
    contributors: &[(u8, PvpContributorKind, bool)],
) -> TestResult<CharacterDeathRequest<10>> {
    let mut request = pvp_death_request(tag)?;
    request.pvp_contributors = contributors
        .iter()
        .map(|&(seed, kind, unjustified)| {
            Ok(PvpDeathContributor {
                character_id: character(seed)?,
                kind,
                unjustified,
            })
        })
        .collect::<TestResult<_>>()?;
    Ok(request)
}

#[test]
fn unjustified_kills_raise_a_white_then_red_skull_that_the_world_job_clears() -> TestResult {
    run_with_initialization("pvp_death_skulls", false, async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        seed_root(h, OTHER, WORLD).await?;
        h.root
            .write_ahead_pvp_deadlines(
                &authority,
                &h.node,
                fence(1)?,
                vec![PvpWriteAhead {
                    character_id: character(CHARACTER)?,
                    kinds: kinds(true, false, false),
                }],
            )
            .await
            .map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        for tag in 90..=92 {
            ledger(h, OTHER, 1).await?;
            let request = request(tag, &[(OTHER, PvpContributorKind::Damage, true)])?;
            let committed = h
                .root
                .settle_player_pvp_death(
                    &sequencer,
                    &authority,
                    &h.node,
                    fence(1)?,
                    request.clone(),
                )
                .await
                .map_err(debug)?;
            assert_eq!(h.count("game_character_pvp_ledger").await?, 0);
            assert_eq!(
                h.count("game_character_unjustified_points").await?,
                i64::from(tag - 89)
            );
            assert_eq!(
                h.count("game_character_revenge_marks").await?,
                i64::from(tag - 89)
            );
            let state: (i16, bool, bool, bool) = sqlx::query_as(
                "SELECT skull, white_skull_until >= kill_block_until, \
                 kill_block_until > clock_timestamp() + interval '14 minutes', \
                 COALESCE(skull_until > clock_timestamp() + interval '29 days', FALSE) \
                 FROM game_character_pvp_state WHERE character_id = encode($1,'hex')::uuid",
            )
            .bind(id(OTHER).as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(state.0, if tag == 92 { 1 } else { 0 });
            assert!(state.1 && state.2);
            if tag == 92 {
                assert!(state.3);
            }
            assert_eq!(
                h.root
                    .commit_character_death(
                        &authority,
                        &h.node,
                        fence(committed.original_character_revision.get())?,
                        request.clone()
                    )
                    .await
                    .map_err(debug)?,
                CharacterDeathOutcome::AlreadyCommitted(committed.clone())
            );
            let mut conflicting = request;
            conflicting.pvp_contributors[0].unjustified = false;
            assert!(matches!(
                h.root
                    .commit_character_death(
                        &authority,
                        &h.node,
                        fence(committed.original_character_revision.get())?,
                        conflicting
                    )
                    .await,
                Err(CharacterProgressionError::ConflictingOccurrence)
            ));
        }
        // Time travel is fixture-only; cleanup itself runs under the real scoped owner.
        seed_skull_fixture(h, OTHER, "-1 microsecond").await?;
        let lane = SpellLane::new(
            crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        );
        let permit = lane.acquire().await.map_err(|_| "spell lane unresolved")?;
        let cleaned = h
            .root
            .drain_pvp_expiry(&permit, &authority, &h.node, scope()?, 1)
            .await
            .map_err(debug)?;
        assert_eq!(cleaned.skulls, 1);
        let skull: (i16, Option<String>) = sqlx::query_as("SELECT skull, skull_until::text FROM game_character_pvp_state WHERE character_id = encode($1,'hex')::uuid")
            .bind(id(OTHER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(skull, (0, None));
        Ok(())
    })
}

#[test]
fn pvp_death_rejects_mismatched_compact_evidence_and_rolls_back_every_consequence() -> TestResult {
    run_with_initialization("pvp_death_reject", false, async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        seed_root(h, OTHER, WORLD).await?;
        h.root
            .write_ahead_pvp_deadlines(
                &authority,
                &h.node,
                fence(1)?,
                vec![PvpWriteAhead {
                    character_id: character(CHARACTER)?,
                    kinds: kinds(true, false, false),
                }],
            )
            .await
            .map_err(debug)?;
        ledger(h, OTHER, 2).await?;
        let sequencer = CharacterRevisionSequencer::new();
        assert!(matches!(
            h.root
                .settle_player_pvp_death(
                    &sequencer,
                    &authority,
                    &h.node,
                    fence(1)?,
                    request(95, &[(OTHER, PvpContributorKind::Damage, true)])?
                )
                .await,
            Err(CharacterProgressionError::InvalidInput)
        ));
        for table in [
            "game_character_death_receipts",
            "game_character_unjustified_points",
            "game_character_revenge_marks",
            "game_character_pending_respawns",
        ] {
            assert_eq!(h.count(table).await?, 0, "{table}");
        }
        assert_eq!(h.count("game_character_pvp_ledger").await?, 1);
        assert_eq!(
            h.count("game_character_pvp_state").await?,
            1,
            "new killer state rolled back"
        );
        let committed = h
            .root
            .settle_player_pvp_death(
                &sequencer,
                &authority,
                &h.node,
                fence(1)?,
                request(95, &[(OTHER, PvpContributorKind::Assist, true)])?,
            )
            .await
            .map_err(debug)?;
        let amount: i32 =
            sqlx::query_scalar("SELECT points_milli FROM game_character_unjustified_points")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(amount, 500);
        assert_eq!(committed.experience_lost.get(), 0);
        Ok(())
    })
}

#[path = "pvp_death_validation_postgres_cases.rs"]
mod validation;
