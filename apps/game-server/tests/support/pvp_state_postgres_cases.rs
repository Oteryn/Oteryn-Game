//! PVP-1 PvP state on PostgreSQL 17.6 (PARTY-PVP-0 §6.2, §8.1; migration 0081).
//! The parent harness provides the migrations and an independently seeded current session.
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, fence, id, runtime,
};
use crate::domain::CharacterId;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::pvp_state::{
    PvpDeadlineKinds, PvpSkull, PvpWriteAhead, cleanup_pvp_rows_in_transaction,
};

const OTHER: u8 = 60;
const THIRD: u8 = 61;
const FOREIGN: u8 = 62;
const FOREIGN_WORLD: u8 = 44;

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, tag, true).await?;
        let outcome = body(&harness).await;
        harness.cleanup().await?;
        outcome
    })
}

fn character(seed: u8) -> TestResult<CharacterId> {
    Ok(CharacterId::from_bytes(id(seed)).map_err(debug)?)
}

/// A new Character root copied from the harness Character, in `world`.
async fn seed_root(h: &Harness, seed: u8, world: u8) -> TestResult {
    sqlx::query(
        "INSERT INTO game_character_roots (character_id, account_id, world_id, lifecycle, \
            character_revision, profile_revision, ruleset_revision, content_revision, \
            starter_template_revision) \
         SELECT encode($2,'hex')::uuid, account_id, encode($3,'hex')::uuid, lifecycle, \
                character_revision, profile_revision, ruleset_revision, content_revision, \
                starter_template_revision \
           FROM game_character_roots WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .bind(id(seed).as_slice())
    .bind(id(world).as_slice())
    .execute(&h.pool)
    .await?;
    Ok(())
}

fn kinds(logout_block: bool, pz_block: bool, white_skull: bool) -> PvpDeadlineKinds {
    PvpDeadlineKinds {
        logout_block,
        pz_block,
        white_skull,
    }
}

/// `deadline - now` in ms of one column, `None` when unset.
async fn remaining(h: &Harness, seed: u8, column: &str) -> TestResult<Option<i64>> {
    let sql = format!(
        "SELECT (extract(epoch FROM ({column} - clock_timestamp())) * 1000)::bigint \
           FROM game_character_pvp_state WHERE character_id = encode($1,'hex')::uuid"
    );
    Ok(
        sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .bind(id(seed).as_slice())
            .fetch_optional(&h.pool)
            .await?
            .flatten(),
    )
}

async fn rows(h: &Harness) -> TestResult<i64> {
    h.count("game_character_pvp_state").await
}

/// §8.1: the write-ahead stores clock + 70 s for exactly the named columns of both Characters
/// in one transaction, creates missing rows, only raises, and the admission read returns the
/// time remaining; the root's CharacterRevision does not move.
#[test]
fn write_ahead_raises_deadlines_and_reads_back_remaining_time() -> TestResult {
    run("pvp_write_ahead", async |h| {
        seed_root(h, OTHER, WORLD).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        assert_eq!(
            h.root.read_pvp_state(&authority, &h.node, fence(1)?).await.map_err(debug)?,
            None
        );
        h.root
            .write_ahead_pvp_deadlines(
                &authority,
                &h.node,
                fence(1)?,
                vec![
                    PvpWriteAhead { character_id: character(OTHER)?, kinds: kinds(false, false, true) },
                    PvpWriteAhead { character_id: character(CHARACTER)?, kinds: kinds(true, true, false) },
                ],
            )
            .await
            .map_err(debug)?;
        assert_eq!(rows(h).await?, 2);
        let stored = h
            .root
            .read_pvp_state(&authority, &h.node, fence(1)?)
            .await
            .map_err(debug)?
            .expect("row");
        assert_eq!(stored.skull, PvpSkull::None);
        assert_eq!(stored.revision, 1);
        assert!(!stored.adventurer_forfeited);
        for ms in [stored.logout_block_remaining_ms, stored.pz_block_remaining_ms] {
            let ms = ms.expect("set");
            assert!((60_000..=70_000).contains(&ms), "{ms}");
        }
        assert_eq!(stored.white_skull_remaining_ms, None);
        assert_eq!(stored.kill_block_remaining_ms, None);
        // The target's row holds only the white skull.
        assert!(remaining(h, OTHER, "white_skull_until").await?.is_some());
        assert_eq!(remaining(h, OTHER, "logout_block_until").await?, None);

        // A second write-ahead raises the named column and keeps the rest.
        let before = remaining(h, CHARACTER, "logout_block_until").await?.expect("set");
        h.root
            .write_ahead_pvp_deadlines(
                &authority,
                &h.node,
                fence(1)?,
                vec![PvpWriteAhead { character_id: character(CHARACTER)?, kinds: kinds(true, false, false) }],
            )
            .await
            .map_err(debug)?;
        assert!(remaining(h, CHARACTER, "logout_block_until").await?.expect("set") >= before - 50);
        assert!(remaining(h, CHARACTER, "pz_block_until").await?.is_some());
        let second = h
            .root
            .read_pvp_state(&authority, &h.node, fence(1)?)
            .await
            .map_err(debug)?
            .expect("row");
        assert_eq!(second.revision, 2);
        // No CharacterRevision advance (§6.2).
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}

/// §8.1 "never lowered": the database refuses any update that lowers or clears a deadline, an
/// update that skips a revision, and a state row of another World.
#[test]
fn the_database_never_lowers_a_deadline() -> TestResult {
    run("pvp_never_lower", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        h.root
            .write_ahead_pvp_deadlines(
                &authority,
                &h.node,
                fence(1)?,
                vec![PvpWriteAhead { character_id: character(CHARACTER)?, kinds: kinds(true, true, true) }],
            )
            .await
            .map_err(debug)?;
        for column in ["logout_block_until", "pz_block_until", "white_skull_until"] {
            for value in ["NULL", "clock_timestamp()"] {
                let sql = format!(
                    "UPDATE game_character_pvp_state SET {column} = {value}, revision = revision + 1"
                );
                assert!(
                    sqlx::query(sqlx::AssertSqlSafe(sql)).execute(&h.pool).await.is_err(),
                    "{column} = {value}"
                );
            }
        }
        // A deadline may be raised, and an unset one set, but the revision advances by one.
        assert!(
            sqlx::query("UPDATE game_character_pvp_state SET kill_block_until = clock_timestamp(), revision = revision + 2")
                .execute(&h.pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("UPDATE game_character_pvp_state SET kill_block_until = clock_timestamp(), revision = revision + 1")
                .execute(&h.pool)
                .await
                .is_ok()
        );
        // A row of another World than its Character's, and a skull without an end, are refused.
        seed_root(h, FOREIGN, FOREIGN_WORLD).await?;
        assert!(
            sqlx::query("INSERT INTO game_character_pvp_state (character_id, world_id) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid)")
                .bind(id(FOREIGN).as_slice())
                .bind(id(WORLD).as_slice())
                .execute(&h.pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("UPDATE game_character_pvp_state SET skull = 1, revision = revision + 1")
                .execute(&h.pool)
                .await
                .is_err()
        );
        Ok(())
    })
}

/// A refused write-ahead writes nothing: invalid target sets, a target of another World, a stale
/// fence.
#[test]
fn refused_write_aheads_write_nothing() -> TestResult {
    run("pvp_refused", async |h| {
        seed_root(h, OTHER, WORLD).await?;
        seed_root(h, THIRD, WORLD).await?;
        seed_root(h, FOREIGN, FOREIGN_WORLD).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let write = async |fence, targets: Vec<(u8, PvpDeadlineKinds)>| -> TestResult<
            Result<(), CharacterProgressionError>,
        > {
            let mut list = Vec::new();
            for (seed, kinds) in targets {
                list.push(PvpWriteAhead { character_id: character(seed)?, kinds });
            }
            Ok(h.root.write_ahead_pvp_deadlines(&authority, &h.node, fence, list).await)
        };
        let all = kinds(true, true, true);
        // The acting Character must be a target; at most two distinct targets; kinds required.
        for targets in [
            vec![],
            vec![(OTHER, all)],
            vec![(CHARACTER, all), (OTHER, all), (THIRD, all)],
            vec![(CHARACTER, all), (CHARACTER, all)],
            vec![(CHARACTER, kinds(false, false, false))],
        ] {
            assert!(matches!(
                write(fence(1)?, targets).await?,
                Err(CharacterProgressionError::InvalidInput)
            ));
        }
        // A target of another World refuses the whole write, the actor's row included.
        assert!(write(fence(1)?, vec![(CHARACTER, all), (FOREIGN, all)]).await?.is_err());
        // A stale lease writes nothing.
        let mut stale = fence(1)?;
        stale.character_lease_generation = 2;
        assert!(write(stale, vec![(CHARACTER, all)]).await?.is_err());
        assert_eq!(rows(h).await?, 0);
        Ok(())
    })
}

/// §6.2, §8.3: a victim ledger holds at most 16 full and 64 compact rows, and a compact row keeps
/// no buckets; the World job deletes ledger rows older than 5 minutes, points older than 45 days
/// and expired marks, at most 100 per kind per pass, and only of its World.
#[test]
fn ledger_caps_hold_and_the_world_job_is_bounded() -> TestResult {
    run("pvp_ledger_cleanup", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        h.root
            .write_ahead_pvp_deadlines(
                &authority,
                &h.node,
                fence(1)?,
                vec![PvpWriteAhead { character_id: character(CHARACTER)?, kinds: kinds(true, false, false) }],
            )
            .await
            .map_err(debug)?;
        // 81 contributors of the victim's World, 100 ledger rows of an old time.
        for seed in 100..=181u8 {
            seed_root(h, seed, WORLD).await?;
        }
        let insert = async |seed: u8, compact: bool, age: &str| {
            let sql = format!(
                "INSERT INTO game_character_pvp_ledger (victim_character_id, contributor_character_id, \
                    buckets, last_at, compact, compact_kind) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, '[]', \
                    clock_timestamp() - interval '{age}', $3, CASE WHEN $3 THEN 1 END)"
            );
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .bind(id(CHARACTER).as_slice())
                .bind(id(seed).as_slice())
                .bind(compact)
                .execute(&h.pool)
                .await
        };
        for seed in 100..116u8 {
            insert(seed, false, "10 minutes").await?;
        }
        assert!(insert(116, false, "10 minutes").await.is_err(), "17th full row");
        for seed in 117..181u8 {
            insert(seed, true, "10 minutes").await?;
        }
        assert!(insert(181, true, "10 minutes").await.is_err(), "65th compact row");
        // Demoting a 65th row to compact is refused, and a compact row keeps no buckets.
        assert!(
            sqlx::query("UPDATE game_character_pvp_ledger SET compact = TRUE, compact_kind = 1 WHERE contributor_character_id = encode($1,'hex')::uuid")
                .bind(id(100).as_slice())
                .execute(&h.pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("UPDATE game_character_pvp_ledger SET buckets = '[{}]' WHERE compact")
                .execute(&h.pool)
                .await
                .is_err()
        );
        // Points and marks need a death receipt: write them as the owner with FK triggers off.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
        for n in 0..101u8 {
            let death = [n, 1, 2, 3, 4, 5, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n];
            sqlx::query(
                "INSERT INTO game_character_unjustified_points (character_id, death_occurrence_id, \
                    victim_character_id, points_milli, committed_at) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, 1000, \
                    clock_timestamp() - interval '46 days')",
            )
            .bind(id(CHARACTER).as_slice())
            .bind(death.as_slice())
            .bind(id(OTHER).as_slice())
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO game_character_revenge_marks (victim_character_id, killer_character_id, \
                death_occurrence_id, created_at, expires_at) \
             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
                clock_timestamp() - interval '8 days', clock_timestamp() - interval '1 day')",
        )
        .bind(id(CHARACTER).as_slice())
        .bind(id(OTHER).as_slice())
        .bind([9_u8, 1, 2, 3, 4, 5, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 9].as_slice())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        // Another World's pass deletes nothing.
        let mut tx = h.pool.begin().await?;
        let other = cleanup_pvp_rows_in_transaction(&mut tx, &id(FOREIGN_WORLD)).await.map_err(debug)?;
        tx.commit().await?;
        assert_eq!((other.points, other.marks, other.ledger_rows), (0, 0, 0));

        let mut tx = h.pool.begin().await?;
        let first = cleanup_pvp_rows_in_transaction(&mut tx, &id(WORLD)).await.map_err(debug)?;
        tx.commit().await?;
        assert_eq!((first.points, first.marks, first.ledger_rows), (100, 1, 80));
        let mut tx = h.pool.begin().await?;
        let second = cleanup_pvp_rows_in_transaction(&mut tx, &id(WORLD)).await.map_err(debug)?;
        tx.commit().await?;
        assert_eq!((second.points, second.marks, second.ledger_rows), (1, 0, 0));
        Ok(())
    })
}
