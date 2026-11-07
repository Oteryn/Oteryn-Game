//! Same-death input, fencing and concurrent-writer rejection through real PostgreSQL.
use super::*;
use crate::character_progression_postgres_cases::join_pvp_writers;

#[test]
fn pvp_death_empty_or_invalid_consequences_never_discard_a_live_ledger() -> TestResult {
    run_with_initialization("pvp_death_inputs", false, async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let initial = h
            .root
            .settle_player_pvp_death(
                &sequencer,
                &authority,
                &h.node,
                fence(1)?,
                pvp_death_request(80)?,
            )
            .await
            .map_err(debug)?;
        let current = initial.committed_character_revision.get();
        h.root
            .consume_pending_respawn(
                &authority,
                &h.node,
                fence(current)?,
                Some(initial.occurrence),
            )
            .await
            .map_err(debug)?;
        seed_root(h, OTHER, WORLD).await?;
        seed_root(h, THIRD, WORLD).await?;
        h.root
            .write_ahead_pvp_deadlines(
                &authority,
                &h.node,
                fence(current)?,
                vec![PvpWriteAhead {
                    character_id: character(CHARACTER)?,
                    kinds: kinds(true, false, false),
                }],
            )
            .await
            .map_err(debug)?;
        ledger(h, OTHER, 1).await?;
        let valid = request(81, &[(OTHER, PvpContributorKind::Damage, true)])?;
        let contributor = valid.pvp_contributors[0];
        let mut invalid = vec![Vec::new(), vec![contributor; 2], vec![contributor; 81]];
        invalid.push(vec![PvpDeathContributor {
            character_id: character(CHARACTER)?,
            ..contributor
        }]);
        invalid.push(vec![PvpDeathContributor {
            character_id: character(THIRD)?,
            ..contributor
        }]);
        invalid.push(vec![PvpDeathContributor {
            kind: PvpContributorKind::Assist,
            ..contributor
        }]);
        invalid.push(vec![
            PvpDeathContributor {
                character_id: character(THIRD)?,
                ..contributor
            },
            contributor,
        ]);
        for contributors in invalid {
            let mut bad = valid.clone();
            bad.pvp_contributors = contributors;
            assert!(matches!(
                h.root
                    .commit_character_death(&authority, &h.node, fence(current)?, bad)
                    .await,
                Err(CharacterProgressionError::InvalidInput)
            ));
        }
        for age in ["1 hour", "-6 minutes"] {
            sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE game_character_pvp_ledger SET last_at = clock_timestamp() + interval '{age}'")))
                .execute(&h.pool).await?;
            assert!(matches!(
                h.root
                    .commit_character_death(&authority, &h.node, fence(current)?, valid.clone())
                    .await,
                Err(CharacterProgressionError::InvalidInput)
            ));
        }
        sqlx::query("UPDATE game_character_pvp_ledger SET last_at = clock_timestamp()")
            .execute(&h.pool)
            .await?;
        let mut stale = fence(current)?;
        stale.character_lease_generation += 1;
        assert!(matches!(
            h.root
                .commit_character_death(&authority, &h.node, stale, valid)
                .await,
            Err(CharacterProgressionError::AuthorityRejected)
        ));
        assert_eq!(h.root_revision().await?, current.to_string());
        assert_eq!(h.count("game_character_death_receipts").await?, 1);
        assert_eq!(h.count("game_character_pvp_ledger").await?, 1);
        assert_eq!(h.count("game_character_unjustified_points").await?, 0);
        assert_eq!(h.count("game_character_revenge_marks").await?, 0);
        Ok(())
    })
}

#[test]
fn pvp_death_damage_split_includes_justified_contributors_and_concurrent_loser_is_typed()
-> TestResult {
    run_with_initialization("pvp_death_split", false, async |h| {
        let seal = h
            .recovery
            .seal_current()
            .map_err(|error| format!("seal: {error:?}"))?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|error| format!("primary authority: {error:?}"))?;
        // Reconcile both holders before adding PVP-1a fixture-only contributor roots.
        // The production writer still checks the independently current victim fences.
        let second = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(second.maintain_ready_once().await?);
        let second_authority = second
            .open_character_authority(&seal)
            .await
            .map_err(|error| format!("secondary authority: {error:?}"))?;
        let sequencer = CharacterRevisionSequencer::new();
        let initial = h
            .root
            .settle_player_pvp_death(
                &sequencer,
                &authority,
                &h.node,
                fence(1)?,
                pvp_death_request(80)?,
            )
            .await
            .map_err(|error| format!("initial PvE death: {error:?}"))?;
        let current = initial.committed_character_revision.get();
        h.root
            .consume_pending_respawn(
                &authority,
                &h.node,
                fence(current)?,
                Some(initial.occurrence),
            )
            .await
            .map_err(|error| format!("consume initial respawn: {error:?}"))?;
        h.root
            .write_ahead_pvp_deadlines(
                &authority,
                &h.node,
                fence(current)?,
                vec![PvpWriteAhead {
                    character_id: character(CHARACTER)?,
                    kinds: kinds(true, false, false),
                }],
            )
            .await
            .map_err(|error| format!("deadline write-ahead: {error:?}"))?;
        let mut contributors = Vec::new();
        for seed in 60..=66 {
            seed_root(h, seed, WORLD).await?;
            let kind = if seed == 66 {
                PvpContributorKind::Assist
            } else {
                PvpContributorKind::Damage
            };
            ledger(h, seed, if seed == 66 { 2 } else { 1 }).await?;
            contributors.push((seed, kind, seed != 65));
        }
        let (a, b) = join_pvp_writers(
            h.root.commit_character_death(
                &authority,
                &h.node,
                fence(current)?,
                request(81, &contributors)?,
            ),
            second.commit_character_death(
                &second_authority,
                &h.node,
                fence(current)?,
                request(82, &contributors)?,
            ),
        )
        .await;
        if a.is_err() && b.is_err() {
            return Err(format!("both concurrent writers failed: {a:?}; {b:?}").into());
        }
        let (winner, loser) = if a.is_ok() { (a, b) } else { (b, a) };
        assert!(matches!(
            winner.map_err(|error| format!("winner: {error:?}"))?,
            CharacterDeathOutcome::Committed(_)
        ));
        assert!(matches!(
            loser,
            Err(CharacterProgressionError::CharacterRevisionMismatch)
        ));
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT points_milli FROM game_character_unjustified_points ORDER BY points_milli",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(rows, vec![(500,), (833,), (833,), (833,), (833,), (833,)]);
        assert_eq!(h.count("game_character_revenge_marks").await?, 6);
        let locks: (i64, i64) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE kill_block_until > clock_timestamp()), \
             count(*) FILTER (WHERE white_skull_until >= kill_block_until) \
             FROM game_character_pvp_state WHERE character_id <> encode($1,'hex')::uuid",
        )
        .bind(id(CHARACTER).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(locks, (7, 6));
        drop(second_authority);
        drop(second);
        Ok(())
    })
}
