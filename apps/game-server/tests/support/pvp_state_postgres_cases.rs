//! PVP-1 PvP state on PostgreSQL 17.6 (PARTY-PVP-0 §6.2, §8.1; migration 0081).
//! The parent harness provides the migrations and an independently seeded current session.
use crate::bestiary_postgres_harness::{
    CHANNEL, CHARACTER, Harness, TestResult, WORLD, configured_admin, context, debug, fence, id,
    runtime, scope,
};
use crate::domain::CharacterId;
use crate::durability::character_progression::CharacterProgressionError;
use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::pvp_state::{
    PvpCleanup, PvpDeadlineKinds, PvpDeathConsequences, PvpDeathContributor, PvpSkull,
    PvpWriteAhead, cleanup_pvp_rows_in_transaction,
};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

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

/// A UUIDv7 death occurrence of tag `n`, distinct from the harness ids.
fn occurrence(n: u8) -> [u8; 16] {
    [n, 9, 2, 3, 4, 5, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
}

/// Runs `sql` as the owner inside a fixture death transaction of `victim` (0123): a receipt
/// inserted with triggers off and the victim's ledger naming `contributor` now. Commits when
/// `sql` succeeds and reports whether it did.
async fn in_death_of(h: &Harness, n: u8, victim: u8, contributor: u8, sql: &str) -> TestResult<bool> {
    let mut tx = h.pool.begin().await?;
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
    .bind(occurrence(n).as_slice())
    .bind(id(victim).as_slice())
    .bind(2000 + i64::from(n))
    .bind(id(WORLD).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query("SET LOCAL session_replication_role = origin")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_pvp_ledger (victim_character_id, contributor_character_id, \
            buckets, last_at) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, '[]', clock_timestamp()) \
         ON CONFLICT DO NOTHING",
    )
    .bind(id(victim).as_slice())
    .bind(id(contributor).as_slice())
    .execute(&mut *tx)
    .await?;
    let ok = sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
        .execute(&mut *tx)
        .await
        .is_ok();
    if ok {
        tx.commit().await?;
    } else {
        tx.rollback().await?;
    }
    Ok(ok)
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
        // A deadline may be raised, and an unset one set, but the revision advances by one. The
        // kill block and the skull are written only in a victim's death transaction (0123).
        seed_root(h, OTHER, WORLD).await?;
        seed_root(h, THIRD, WORLD).await?;
        sqlx::query("INSERT INTO game_character_pvp_state (character_id, world_id) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid)")
            .bind(id(OTHER).as_slice())
            .bind(id(WORLD).as_slice())
            .execute(&h.pool)
            .await?;
        let hex: String = id(CHARACTER).iter().map(|byte| format!("{byte:02x}")).collect();
        let kill_block = format!(
            "UPDATE game_character_pvp_state SET kill_block_until = clock_timestamp(), \
             revision = revision + 1 WHERE character_id = '{hex}'::uuid"
        );
        let kill_block = kill_block.as_str();
        assert!(
            sqlx::query(sqlx::AssertSqlSafe(kill_block.to_owned()))
                .execute(&h.pool)
                .await
                .is_err(),
            "outside a death transaction"
        );
        assert!(
            !in_death_of(h, 1, OTHER, THIRD, kill_block).await?,
            "a Character outside the victim's ledger"
        );
        assert!(
            !in_death_of(h, 1, OTHER, CHARACTER, &kill_block.replace("+ 1 WHERE", "+ 2 WHERE")).await?,
            "a skipped revision"
        );
        assert!(in_death_of(h, 1, OTHER, CHARACTER, kill_block).await?);
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
            !in_death_of(
                h,
                2,
                OTHER,
                CHARACTER,
                "UPDATE game_character_pvp_state SET skull = 1, revision = revision + 1"
            )
            .await?
        );
        // An unexpired skull is not cleared, shortened or downgraded; forfeiture is permanent.
        assert!(
            in_death_of(
                h,
                2,
                OTHER,
                CHARACTER,
                &kill_block.replace(
                    "kill_block_until = clock_timestamp()",
                    "skull = 2, skull_until = clock_timestamp() + interval '1 hour', \
                     adventurer_forfeited = TRUE"
                )
            )
            .await?
        );
        for change in [
            "skull = 0, skull_until = NULL",
            "skull = 1",
            "skull = 2, skull_until = skull_until - interval '1 minute'",
            "skull_until = skull_until - interval '1 minute'",
            "adventurer_forfeited = FALSE",
        ] {
            assert!(
                !in_death_of(
                    h,
                    3,
                    OTHER,
                    CHARACTER,
                    &kill_block.replace("kill_block_until = clock_timestamp()", change)
                )
                .await?,
                "{change}"
            );
        }
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
        // Contribution evidence: the victim's ledger names its recipients within the window.
        sqlx::query("INSERT INTO game_character_pvp_state (character_id, world_id) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid)")
            .bind(id(OTHER).as_slice())
            .bind(id(WORLD).as_slice())
            .execute(&h.pool)
            .await?;
        let evidence = async |contributor: u8, age: &str| {
            let sql = format!(
                "INSERT INTO game_character_pvp_ledger (victim_character_id, contributor_character_id, \
                    buckets, last_at, compact, compact_kind) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, '[]', \
                    clock_timestamp() - interval '{age}', FALSE, NULL)"
            );
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .bind(id(OTHER).as_slice())
                .bind(id(contributor).as_slice())
                .execute(&h.pool)
                .await
        };
        evidence(CHARACTER, "1 second").await?;
        evidence(THIRD, "10 minutes").await?;
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
        // A recipient whose ledger row is outside the window is refused; refreshed, it is accepted.
        let mut tx = h.pool.begin().await?;
        receipt(&mut tx, 110, OTHER).await?;
        assert!(point(&mut tx, THIRD, OTHER, 110).await.is_err());
        tx.rollback().await?;
        sqlx::query("UPDATE game_character_pvp_ledger SET last_at = clock_timestamp() WHERE victim_character_id = encode($1,'hex')::uuid AND contributor_character_id = encode($2,'hex')::uuid")
            .bind(id(OTHER).as_slice())
            .bind(id(THIRD).as_slice())
            .execute(&h.pool)
            .await?;
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
        // A point is never updated, and a mark is not updated here.
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
        // Closing a mark belongs to the avenging death transaction (PVP-DEATH-1): not here.
        let mut tx = h.pool.begin().await?;
        receipt(&mut tx, 114, CHARACTER).await?;
        assert!(
            sqlx::query("UPDATE game_character_revenge_marks SET avenged_at = now()")
                .execute(&mut *tx)
                .await
                .is_err()
        );
        tx.rollback().await?;
        // The runtime role writes no forfeiture yet, and skull, kill block, points and marks only
        // inside its victim's death transaction (0123 and 0081 guards).
        for sql in [
            "UPDATE game_character_pvp_state SET skull = 2, skull_until = now() + interval '1 day'",
            "UPDATE game_character_pvp_state SET adventurer_forfeited = TRUE",
            "UPDATE game_character_pvp_state SET kill_block_until = now() + interval '1 day'",
            "INSERT INTO game_character_unjustified_points (character_id, death_occurrence_id, \
                victim_character_id, points_milli) SELECT character_id, character_id, character_id, 1 \
                FROM game_character_pvp_state",
            "INSERT INTO game_character_revenge_marks (victim_character_id, killer_character_id, \
                death_occurrence_id) SELECT character_id, character_id, character_id \
                FROM game_character_pvp_state",
        ] {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            assert!(sqlx::query(sql).execute(&mut *tx).await.is_err(), "{sql}");
            tx.rollback().await?;
        }
        // Unexpired rows are not deleted, by any runtime path.
        for table in [
            "game_character_unjustified_points",
            "game_character_revenge_marks",
        ] {
            assert!(
                sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table}")))
                    .execute(&h.pool)
                    .await
                    .is_err(),
                "{table}"
            );
        }
        assert!(
            sqlx::query("DELETE FROM game_character_pvp_ledger WHERE victim_character_id = encode($1,'hex')::uuid")
                .bind(id(OTHER).as_slice())
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

/// Global thresholds of levels 1-10, as in the DEATH-1 writer cases.
const GLOBAL: [i64; 11] = [0, 100, 200, 400, 800, 1500, 2600, 4200, 6400, 9300, 13000];

fn death(n: u8) -> TestResult<CharacterDeathRequest<10>> {
    let mut thresholds = [LevelThreshold {
        level: 1,
        minimum_experience: ExactI64::new(0),
    }; 10];
    for (index, threshold) in thresholds.iter_mut().enumerate() {
        *threshold = LevelThreshold {
            level: u32::try_from(index + 1)?,
            minimum_experience: ExactI64::new(GLOBAL[index]),
        };
    }
    Ok(CharacterDeathRequest {
        occurrence: PlayerDeathOccurrence::from_bytes(occurrence(n)).map_err(debug)?,
        context: context(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: FiniteProgressionPolicy {
            context: context(),
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds,
            terminal_exclusive_experience: ExactI64::new(GLOBAL[10]),
            death_loss_numerator: 1,
            death_loss_denominator: 1,
            death_loss_rounding: RoundingMode::Floor,
        },
        held_blessings: Vec::new(),
        death_cell: DeathCell {
            world_id: crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,
            channel_id: crate::foundation::ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            spatial_position: vec![1, 2, 3, 7],
            map_revision: "map-1".into(),
        },
        respawn_position: b"temple:thais".to_vec(),
    })
}

/// The killer's stored row: skull, and the remaining ms of skull, kill block and white skull.
async fn killer_row(h: &Harness) -> TestResult<(i16, Option<i64>, Option<i64>, Option<i64>)> {
    Ok(sqlx::query_as(
        "SELECT skull, \
            (extract(epoch FROM (skull_until - clock_timestamp())) * 1000)::bigint, \
            (extract(epoch FROM (kill_block_until - clock_timestamp())) * 1000)::bigint, \
            (extract(epoch FROM (white_skull_until - clock_timestamp())) * 1000)::bigint \
           FROM game_character_pvp_state WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(OTHER).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

/// The killer's point rows and their sum, its revenge marks and the victim's ledger rows.
async fn consequences(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_unjustified_points \
                  WHERE character_id = encode($1,'hex')::uuid), \
                (SELECT coalesce(sum(points_milli), 0)::bigint FROM game_character_unjustified_points \
                  WHERE character_id = encode($1,'hex')::uuid), \
                (SELECT count(*) FROM game_character_revenge_marks \
                  WHERE killer_character_id = encode($1,'hex')::uuid \
                    AND victim_character_id = encode($2,'hex')::uuid), \
                (SELECT count(*) FROM game_character_pvp_ledger \
                  WHERE victim_character_id = encode($2,'hex')::uuid)",
    )
    .bind(id(OTHER).as_slice())
    .bind(id(CHARACTER).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

/// PVP-1b end to end on the real death writer (PARTY-PVP-0 §6.2, §9; PVP-0 §6.1; D1443): the
/// killer A (`OTHER`) kills the victim B (the harness Character) unjustified three times within
/// a day. Each death transaction writes A's kill block, white skull, point and revenge mark and
/// deletes B's ledger; the third reaches the red threshold (3000 milli-points in 24 h) and sets
/// a 30-day red skull. The skull survives A's relog (the admission read), and the World job
/// clears it only after its end.
#[test]
fn unjustified_kills_raise_a_white_then_red_skull_that_the_world_job_clears() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "pvp_skull_deaths", false).await?;
        let outcome = skull_deaths(&harness).await;
        harness.cleanup().await?;
        outcome
    })
}

async fn skull_deaths(h: &Harness) -> TestResult {
    // A level-9 victim at 6500 experience, as in the DEATH-1 writer cases.
    sqlx::query(
        "INSERT INTO game_character_progression_state VALUES \
         (encode($1,'hex')::uuid,1,9,6500,'profile-1','ruleset-1','content-1',\
          'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
    )
    .bind(id(CHARACTER).as_slice())
    .execute(&h.pool)
    .await?;
    seed_root(h, OTHER, WORLD).await?;
    sqlx::query("INSERT INTO game_character_pvp_state (character_id, world_id) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid)")
        .bind(id(CHARACTER).as_slice())
        .bind(id(WORLD).as_slice())
        .execute(&h.pool)
        .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let sequencer = CharacterRevisionSequencer::new();
    let killer = PvpDeathConsequences {
        contributors: vec![PvpDeathContributor {
            character_id: character(OTHER)?,
            damage: true,
            justified: false,
        }],
    };
    // The runtime's contribution ledger (PVP-RT-1) is a fixture: A damaged B just now.
    let hit = async || -> TestResult {
        sqlx::query(
            "INSERT INTO game_character_pvp_ledger (victim_character_id, contributor_character_id, \
                buckets, last_at) \
             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, '[]', clock_timestamp())",
        )
        .bind(id(CHARACTER).as_slice())
        .bind(id(OTHER).as_slice())
        .execute(&h.pool)
        .await?;
        Ok(())
    };
    for kill in 1..=3u8 {
        hit().await?;
        let committed = h
            .root
            .settle_player_pvp_death(
                &sequencer,
                &authority,
                &h.node,
                fence(u64::from(kill))?,
                death(kill)?,
                killer.clone(),
            )
            .await
            .map_err(debug)?;
        assert_eq!(committed.committed_character_revision.get(), u64::from(kill) + 1);
        // The placement consumed the respawn (DEATH-2), so the victim can die again.
        sqlx::query("DELETE FROM game_character_pending_respawns")
            .execute(&h.pool)
            .await?;
        let (skull, skull_ms, kill_ms, white_ms) = killer_row(h).await?;
        let kill_ms = kill_ms.ok_or("kill block")?;
        assert!((14 * 60_000..=15 * 60_000).contains(&kill_ms), "{kill_ms}");
        assert!(white_ms.ok_or("white skull")? >= kill_ms - 1_000);
        assert_eq!(
            consequences(h).await?,
            (i64::from(kill), 1000 * i64::from(kill), i64::from(kill), 0)
        );
        if kill < 3 {
            assert_eq!((skull, skull_ms), (0, None), "white skull only after kill {kill}");
        } else {
            assert_eq!(skull, 1, "red skull at 3000 milli-points in 24 h");
            let skull_ms = skull_ms.ok_or("red skull end")?;
            assert!((30 * 86_400_000 - 60_000..=30 * 86_400_000).contains(&skull_ms));
        }
    }
    // The killer's CharacterRevision does not move (§6.2 composition rule 1 amendment).
    let killer_revision: String = sqlx::query_scalar(
        "SELECT character_revision::text FROM game_character_roots WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(OTHER).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(killer_revision, "1");
    // An exact replay writes nothing more; other contributors conflict; a death whose
    // contributors are not the victim's ledger is refused and writes nothing.
    h.root
        .settle_player_pvp_death(&sequencer, &authority, &h.node, fence(4)?, death(3)?, killer.clone())
        .await
        .map_err(debug)?;
    assert!(
        h.root
            .settle_player_death(&sequencer, &authority, &h.node, fence(4)?, death(3)?)
            .await
            .is_err()
    );
    assert!(
        h.root
            .settle_player_pvp_death(&sequencer, &authority, &h.node, fence(4)?, death(4)?, killer.clone())
            .await
            .is_err()
    );
    assert_eq!(consequences(h).await?, (3, 3000, 3, 0));
    assert_eq!(h.count("game_character_death_receipts").await?, 3);

    // A's relog: the session of the account now plays A on a new connection (fixture), and the
    // fenced admission read returns the red skull.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    for sql in [
        "UPDATE game_durability_reconnect_sessions SET character_id = encode($1,'hex')::uuid, \
            current_generation = 2",
        "UPDATE game_durability_admission_account_guards SET presence_character_id = encode($1,'hex')::uuid",
        "UPDATE game_durability_admission_character_guards SET character_id = encode($1,'hex')::uuid",
    ] {
        sqlx::query(sql).bind(id(OTHER).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let relogged = crate::durability::character_progression::CurrentCharacterGameplayFence {
        character_id: character(OTHER)?,
        connection_generation: crate::foundation::ConnectionGeneration::new(2).map_err(debug)?,
        ..fence(1)?
    };
    let stored = h
        .root
        .read_pvp_state(&authority, &h.node, relogged)
        .await
        .map_err(debug)?
        .ok_or("killer row")?;
    assert_eq!(stored.skull, PvpSkull::Red);
    assert!(stored.skull_remaining_ms.ok_or("red skull end")? > 29 * 86_400_000);
    assert!(stored.kill_block_remaining_ms.is_some());

    // The registered World job (`drain_pvp_expiry`): an unexpired skull stays; after its end
    // (the 30 days are a fixture) one pass clears it.
    let lane = crate::durability::spell_owner_commit::SpellLane::new(
        crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
    );
    let permit = lane.acquire().await.map_err(|_| "spell lane unresolved")?;
    let pass = h
        .root
        .drain_pvp_expiry(&permit, &authority, &h.node, scope()?, 1)
        .await
        .map_err(debug)?;
    assert_eq!(pass.skulls, 0);
    assert_eq!(killer_row(h).await?.0, 1);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_character_pvp_state SET skull_until = clock_timestamp() - interval '1 second' \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(OTHER).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let pass = h
        .root
        .drain_pvp_expiry(&permit, &authority, &h.node, scope()?, 1)
        .await
        .map_err(debug)?;
    assert_eq!(pass.skulls, 1);
    assert_eq!(killer_row(h).await?.0, 0);
    let stored = h
        .root
        .read_pvp_state(&authority, &h.node, relogged)
        .await
        .map_err(debug)?
        .ok_or("killer row")?;
    assert_eq!((stored.skull, stored.skull_remaining_ms), (PvpSkull::None, None));
    // The points stay for their 45 days (`PARTYPVP0-RL-23`).
    assert_eq!(consequences(h).await?, (3, 3000, 3, 0));
    Ok(())
}
