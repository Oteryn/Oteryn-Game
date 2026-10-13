//! PVP-1 PvP state on PostgreSQL 17.6 (PARTY-PVP-0 §6.2, §8.1; migration 0081).
//! The parent harness provides the migrations and an independently seeded current session.
use crate::bestiary_postgres_harness::{
    CHANNEL, CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, fence, id, runtime,
    scope,
};
use crate::domain::CharacterId;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::pvp_state::{
    PvpCleanup, PvpDeadlineKinds, PvpSkull, PvpWriteAhead, cleanup_pvp_rows_in_transaction,
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
            starter_template_revision, name) \
         SELECT encode($2,'hex')::uuid, account_id, encode($3,'hex')::uuid, lifecycle, \
                character_revision, profile_revision, ruleset_revision, content_revision, \
                starter_template_revision, \
                'Pvp' || chr(97 + $4::int / 26) || chr(97 + $4::int % 26) \
           FROM game_character_roots WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .bind(id(seed).as_slice())
    .bind(id(world).as_slice())
    .bind(i32::from(seed))
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
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(id(seed).as_slice())
        .fetch_optional(&h.pool)
        .await?
        .flatten())
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
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        seed_root(h, OTHER, WORLD).await?;
        assert_eq!(
            h.root
                .read_pvp_state(&authority, &h.node, fence(1)?)
                .await
                .map_err(debug)?,
            None
        );
        h.root
            .write_ahead_pvp_deadlines(
                &authority,
                &h.node,
                fence(1)?,
                vec![
                    PvpWriteAhead {
                        character_id: character(OTHER)?,
                        kinds: kinds(false, false, true),
                    },
                    PvpWriteAhead {
                        character_id: character(CHARACTER)?,
                        kinds: kinds(true, true, false),
                    },
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
        for ms in [
            stored.logout_block_remaining_ms,
            stored.pz_block_remaining_ms,
        ] {
            let ms = ms.expect("set");
            assert!((60_000..=70_000).contains(&ms), "{ms}");
        }
        assert_eq!(stored.white_skull_remaining_ms, None);
        assert_eq!(stored.kill_block_remaining_ms, None);
        // The target's row holds only the white skull.
        assert!(remaining(h, OTHER, "white_skull_until").await?.is_some());
        assert_eq!(remaining(h, OTHER, "logout_block_until").await?, None);

        // A second write-ahead raises the named column and keeps the rest.
        let before = remaining(h, CHARACTER, "logout_block_until")
            .await?
            .expect("set");
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
        assert!(
            remaining(h, CHARACTER, "logout_block_until")
                .await?
                .expect("set")
                >= before - 50
        );
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
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        h.root
            .write_ahead_pvp_deadlines(
                &authority,
                &h.node,
                fence(1)?,
                vec![PvpWriteAhead {
                    character_id: character(CHARACTER)?,
                    kinds: kinds(true, true, true),
                }],
            )
            .await
            .map_err(debug)?;
        for column in ["logout_block_until", "pz_block_until", "white_skull_until"] {
            for value in ["NULL", "clock_timestamp()"] {
                let sql = format!(
                    "UPDATE game_character_pvp_state SET {column} = {value}, revision = revision + 1"
                );
                assert!(
                    sqlx::query(sqlx::AssertSqlSafe(sql))
                        .execute(&h.pool)
                        .await
                        .is_err(),
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
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        seed_root(h, OTHER, WORLD).await?;
        seed_root(h, THIRD, WORLD).await?;
        seed_root(h, FOREIGN, FOREIGN_WORLD).await?;
        let write = async |fence,
                           targets: Vec<(u8, PvpDeadlineKinds)>|
               -> TestResult<Result<(), CharacterProgressionError>> {
            let mut list = Vec::new();
            for (seed, kinds) in targets {
                list.push(PvpWriteAhead {
                    character_id: character(seed)?,
                    kinds,
                });
            }
            Ok(h.root
                .write_ahead_pvp_deadlines(&authority, &h.node, fence, list)
                .await)
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
        assert!(
            write(fence(1)?, vec![(CHARACTER, all), (FOREIGN, all)])
                .await?
                .is_err()
        );
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
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
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
        seed_root(h, OTHER, WORLD).await?;
        seed_root(h, THIRD, WORLD).await?;
        seed_root(h, FOREIGN, FOREIGN_WORLD).await?;
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
        assert!(
            insert(116, false, "10 minutes").await.is_err(),
            "17th full row"
        );
        for seed in 117..181u8 {
            insert(seed, true, "10 minutes").await?;
        }
        assert!(
            insert(181, true, "10 minutes").await.is_err(),
            "65th compact row"
        );
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
        // Points and marks are written by the victim's death transaction: the receipt is a
        // fixture inserted with triggers off, then the consequence is written under the real
        // guards in the same transaction.
        let death_of = |n: u8| [n, 1, 2, 3, 4, 5, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n];
        let receipt = async |tx: &mut sqlx::PgConnection, n: u8, victim: u8| -> TestResult<()> {
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "INSERT INTO game_character_death_receipts (death_occurrence_id, command_binding, \
                    policy_digest, character_id, original_character_revision, \
                    committed_character_revision, level_before, level_after, experience_before, \
                    experience_after, experience_lost, blessings_before, blessings_after, \
                    lost_item_ids, death_world_id, death_channel_id, death_spatial_position, \
                    death_map_revision, respawn_position, death_policy_revision, profile_revision, \
                    ruleset_revision, content_revision, simulation_revision, evidence_revision, \
                    declaration_revision, policy_revision, reward_revision, committed_at) \
                 VALUES (encode($1,'hex')::uuid, '\\x01', decode(repeat('00',32),'hex'), \
                    encode($2,'hex')::uuid, $3, $3 + 1, 1, 1, 0, 0, 0, '{}', '{}', '{}', \
                    encode($4,'hex')::uuid, encode($4,'hex')::uuid, '\\x01', 'm', '\\x01', \
                    'p', 'p', 'p', 'p', 'p', 'p', 'p', 'p', 'p', 0)",
            )
            .bind(death_of(n).as_slice())
            .bind(id(victim).as_slice())
            .bind(1000 + i64::from(n))
            .bind(id(WORLD).as_slice())
            .execute(&mut *tx)
            .await?;
            sqlx::query("SET LOCAL session_replication_role = origin")
                .execute(&mut *tx)
                .await?;
            Ok(())
        };
        // Aged rows are fixtures (triggers off, receipt created in the same transaction): 101
        // points of 46 days and one mark expired a day ago.
        for n in 0..101u8 {
            let mut tx = h.pool.begin().await?;
            receipt(&mut tx, n, OTHER).await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "INSERT INTO game_character_unjustified_points (character_id, death_occurrence_id, \
                    victim_character_id, points_milli, committed_at) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, 1000, \
                    clock_timestamp() - interval '46 days')",
            )
            .bind(id(CHARACTER).as_slice())
            .bind(death_of(n).as_slice())
            .bind(id(OTHER).as_slice())
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        let mut tx = h.pool.begin().await?;
        receipt(&mut tx, 101, CHARACTER).await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO game_character_revenge_marks (victim_character_id, killer_character_id, \
                death_occurrence_id, created_at, expires_at) \
             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
                now() - interval '8 days', now() - interval '1 day')",
        )
        .bind(id(CHARACTER).as_slice())
        .bind(id(OTHER).as_slice())
        .bind(death_of(101).as_slice())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        // The real guards: a point and a mark are written by the death transaction of their
        // victim, in the death World, and the database stamps their anchors.
        const POINT: &str = "INSERT INTO game_character_unjustified_points (character_id, \
            death_occurrence_id, victim_character_id, points_milli, committed_at) \
            VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, 1000, \
            now() - interval '90 days')";
        const MARK: &str = "INSERT INTO game_character_revenge_marks (victim_character_id, \
            killer_character_id, death_occurrence_id, created_at, expires_at) \
            VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
            now() - interval '8 days', now() - interval '7 days')";
        let point = async |tx: &mut sqlx::PgConnection, earner: u8, victim: u8, n: u8| {
            sqlx::query(POINT)
                .bind(id(earner).as_slice())
                .bind(death_of(n).as_slice())
                .bind(id(victim).as_slice())
                .execute(&mut *tx)
                .await
        };
        let mark = async |tx: &mut sqlx::PgConnection, victim: u8, killer: u8, n: u8| {
            sqlx::query(MARK)
                .bind(id(victim).as_slice())
                .bind(id(killer).as_slice())
                .bind(death_of(n).as_slice())
                .execute(&mut *tx)
                .await
        };
        // Refused, each in its own death transaction: another victim's death, a recipient of
        // another World, and the old-receipt replay of a later transaction.
        let mut tx = h.pool.begin().await?;
        receipt(&mut tx, 110, OTHER).await?;
        assert!(point(&mut tx, CHARACTER, THIRD, 110).await.is_err());
        tx.rollback().await?;
        let mut tx = h.pool.begin().await?;
        receipt(&mut tx, 110, OTHER).await?;
        assert!(point(&mut tx, FOREIGN, OTHER, 110).await.is_err());
        tx.rollback().await?;
        let mut tx = h.pool.begin().await?;
        receipt(&mut tx, 110, OTHER).await?;
        assert!(mark(&mut tx, OTHER, FOREIGN, 110).await.is_err());
        tx.rollback().await?;
        let mut tx = h.pool.begin().await?;
        assert!(point(&mut tx, THIRD, OTHER, 0).await.is_err());
        tx.rollback().await?;
        // Accepted: anchors chosen by the caller are replaced by the database clock.
        let mut tx = h.pool.begin().await?;
        receipt(&mut tx, 111, OTHER).await?;
        point(&mut tx, THIRD, OTHER, 111).await?;
        mark(&mut tx, OTHER, CHARACTER, 111).await?;
        tx.commit().await?;
        let stamped: (bool, bool) = sqlx::query_as(
            "SELECT (SELECT committed_at > now() - interval '1 minute' \
                       FROM game_character_unjustified_points WHERE character_id = encode($1,'hex')::uuid), \
                    (SELECT created_at > now() - interval '1 minute' \
                        AND expires_at = created_at + interval '7 days' \
                       FROM game_character_revenge_marks WHERE killer_character_id = encode($2,'hex')::uuid)",
        )
        .bind(id(THIRD).as_slice())
        .bind(id(CHARACTER).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(stamped, (true, true));
        // A point is never updated; a mark changes only `avenged_at`, once.
        assert!(
            sqlx::query("UPDATE game_character_unjustified_points SET points_milli = 1")
                .execute(&h.pool)
                .await
                .is_err()
        );
        for column in [
            "expires_at = expires_at + interval '1 day'",
            "created_at = now()",
        ] {
            assert!(
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE game_character_revenge_marks SET {column} \
                      WHERE killer_character_id = encode($1,'hex')::uuid"
                )))
                .bind(id(CHARACTER).as_slice())
                .execute(&h.pool)
                .await
                .is_err()
            );
        }
        let avenge = "UPDATE game_character_revenge_marks SET avenged_at = now() \
                       WHERE killer_character_id = encode($1,'hex')::uuid";
        sqlx::query(avenge)
            .bind(id(CHARACTER).as_slice())
            .execute(&h.pool)
            .await?;
        assert!(
            sqlx::query(avenge)
                .bind(id(CHARACTER).as_slice())
                .execute(&h.pool)
                .await
                .is_err()
        );

        let lane = crate::durability::spell_owner_commit::SpellLane::new(
            crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        );
        let permit = lane.acquire().await.map_err(|_| "spell lane unresolved")?;
        let pass = async || -> TestResult<PvpCleanup> {
            let mut tx = h.pool.begin().await?;
            let held =
                crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
                    &mut tx,
                    &permit,
                    &h.root,
                    &authority,
                    &h.node,
                    scope()?,
                    1,
                )
                .await
                .map_err(debug)?;
            let cleaned = cleanup_pvp_rows_in_transaction(&mut tx, &held)
                .await
                .map_err(debug)?;
            // The authority is bound to its transaction: another one refuses it.
            let mut other = h.pool.begin().await?;
            assert!(
                cleanup_pvp_rows_in_transaction(&mut other, &held)
                    .await
                    .is_err()
            );
            other.rollback().await?;
            tx.commit().await?;
            Ok(cleaned)
        };
        let first = pass().await?;
        assert_eq!((first.points, first.marks, first.ledger_rows), (100, 1, 80));
        let second = pass().await?;
        assert_eq!((second.points, second.marks, second.ledger_rows), (1, 0, 0));
        Ok(())
    })
}
