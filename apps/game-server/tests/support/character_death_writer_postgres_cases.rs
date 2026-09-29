// DEATH-1a `commit_character_death` / `reconcile_character_death` cases. A
// child of the shared P03 cases, so both PostgreSQL wrappers run them with
// the same harness (fenced session, scope assignment, node incarnation).

use super::{Harness, TestResult, configured_admin, context, fence, id, join_two};
use crate::domain::CharacterId;
use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
use crate::durability::DurabilityRoot;
use crate::durability::character_death::{
    CharacterDeathOutcome, CharacterDeathRequest, DeathCell, PlayerDeathOccurrence,
};
use crate::durability::character_progression::{
    CharacterProgressionError, ExperienceAwardRequest, ExperienceCommitOutcome,
    ExperienceRewardOccurrence,
};
use crate::foundation::{ConnectionGeneration, ScopeOwnershipGeneration};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

/// Hyphenated text of `id(seed)`, as PostgreSQL prints a UUID.
fn uuid(seed: u8) -> String {
    let hex: String = id(seed).iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Global thresholds of levels 1-10 (`50/3 × (L³ − 6L² + 17L − 12)`).
const GLOBAL: [i64; 11] = [0, 100, 200, 400, 800, 1500, 2600, 4200, 6400, 9300, 13000];

fn policy() -> FiniteProgressionPolicy<String, 10> {
    let mut thresholds = [LevelThreshold {
        level: 1,
        minimum_experience: ExactI64::new(0),
    }; 10];
    for (index, threshold) in thresholds.iter_mut().enumerate() {
        *threshold = LevelThreshold {
            level: u32::try_from(index + 1).unwrap_or(u32::MAX),
            minimum_experience: ExactI64::new(GLOBAL[index]),
        };
    }
    FiniteProgressionPolicy {
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
    }
}

fn death(tag: u8, blessings: &[&str]) -> TestResult<CharacterDeathRequest<10>> {
    Ok(CharacterDeathRequest {
        occurrence: PlayerDeathOccurrence::from_bytes(id(tag))
            .map_err(|error| format!("{error:?}"))?,
        context: context(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: policy(),
        held_blessings: blessings.iter().map(|key| (*key).to_owned()).collect(),
        death_cell: DeathCell {
            world_id: crate::foundation::WorldId::decode(&id(42))
                .map_err(|error| format!("{error:?}"))?,
            channel_id: crate::foundation::ChannelId::decode(&id(43))
                .map_err(|error| format!("{error:?}"))?,
            spatial_position: vec![1, 2, 3, 7],
            map_revision: "map-1".into(),
        },
        respawn_position: b"temple:thais".to_vec(),
    })
}

fn award(tag: u8, amount: i64) -> TestResult<ExperienceAwardRequest<10>> {
    Ok(ExperienceAwardRequest {
        occurrence: ExperienceRewardOccurrence::from_bytes(id(tag))
            .map_err(|error| format!("{error:?}"))?,
        amount: ExactI64::new(amount),
        context: context(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: policy(),
    })
}

/// A level-9 Character at 6500 experience, optionally holding blessings
/// (inserted as the test owner: no runtime path inserts blessings yet).
async fn create(admin: String, tag: &str, blessings: &[&str]) -> TestResult<Harness> {
    let harness = Harness::create(admin, tag, false).await?;
    sqlx::query(
        "INSERT INTO game_character_progression_state VALUES \
         (encode($1,'hex')::uuid,1,9,6500,'profile-1','ruleset-1','content-1',\
          'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
    )
    .bind(id(41).as_slice())
    .execute(&harness.pool)
    .await?;
    for blessing in blessings {
        sqlx::query(
            "INSERT INTO game_character_blessings VALUES (encode($1,'hex')::uuid, $2, 'test')",
        )
        .bind(id(41).as_slice())
        .bind(*blessing)
        .execute(&harness.pool)
        .await?;
    }
    Ok(harness)
}

/// Root revision, typed state, receipt counts per kind, held blessings and
/// the pending respawn occurrence.
async fn snapshot(pool: &sqlx::PgPool) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', \
           (SELECT character_revision::text FROM game_character_roots), \
           (SELECT concat_ws(',', character_revision, level, total_experience) \
              FROM game_character_progression_state), \
           (SELECT count(*) FROM game_character_xp_receipts), \
           (SELECT count(*) FROM game_character_death_receipts), \
           (SELECT count(*) FROM game_character_stance_receipts), \
           (SELECT coalesce(string_agg(blessing_key, ',' ORDER BY blessing_key), '-') \
              FROM game_character_blessings), \
           (SELECT coalesce(string_agg(death_occurrence_id::text, ','), '-') \
              FROM game_character_pending_respawns))",
    )
    .fetch_one(pool)
    .await?)
}

async fn consume_pending(pool: &sqlx::PgPool) -> TestResult {
    sqlx::query("DELETE FROM game_character_pending_respawns")
        .execute(pool)
        .await?;
    Ok(())
}

/// One stance toggle (0017) at `original`, with experience and level
/// unchanged; STANCE-1 owns its writer.
async fn toggle_stance(pool: &sqlx::PgPool, tag: u8, original: u64, at: (i64, i64)) -> TestResult {
    let mut tx = pool.begin().await?;
    let committed = original + 1;
    sqlx::query(
        "UPDATE game_character_roots SET character_revision = $2::text::numeric(20,0) \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .bind(committed.to_string())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE game_character_progression_state SET character_revision = $2::text::numeric(20,0) \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .bind(committed.to_string())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_stance_receipts(\
           stance_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, level_before, \
           level_after, experience_before, experience_after, stance_before, stance_after, \
           profile_revision, ruleset_revision, content_revision, simulation_revision, \
           evidence_revision, declaration_revision, policy_revision, reward_revision, \
           committed_at) \
         VALUES (encode($1,'hex')::uuid, $2, $3, encode($4,'hex')::uuid, \
           $5::text::numeric(20,0), $6::text::numeric(20,0), $7, $7, $8, $8, NULL, 'stance-a', \
           'profile-1', 'ruleset-1', 'content-1', 'simulation-1', 'evidence-1', \
           'declaration-1', 'policy-1', 'reward-1', 3)",
    )
    .bind(id(tag).as_slice())
    .bind([tag; 33].as_slice())
    .bind([tag; 32].as_slice())
    .bind(id(41).as_slice())
    .bind(original.to_string())
    .bind(committed.to_string())
    .bind(at.0)
    .bind(at.1)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_stance VALUES \
         (encode($1,'hex')::uuid, 'stance-a', $2::text::numeric(20,0), encode($3,'hex')::uuid)",
    )
    .bind(id(41).as_slice())
    .bind(committed.to_string())
    .bind(id(tag).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

fn committed(
    outcome: CharacterDeathOutcome,
) -> TestResult<crate::durability::character_death::CommittedCharacterDeath> {
    match outcome {
        CharacterDeathOutcome::Committed(committed) => Ok(committed),
        other => Err(format!("death was not newly committed: {other:?}").into()),
    }
}

fn run<F>(body: F) -> TestResult
where
    F: FnOnce(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = TestResult>>>,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(body(admin))
}

#[test]
fn death_commits_one_receipt_replays_reconciles_and_survives_restart() -> TestResult {
    run(|admin| {
        Box::pin(async move {
            let harness = create(admin, "death_replay", &["embrace", "spark"]).await?;
            let seal = harness
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            let request = death(80, &["embrace", "spark"])?;
            let first = committed(
                harness
                    .root
                    .commit_character_death(&authority, &harness.node, fence(1)?, request.clone())
                    .await
                    .map_err(|error| format!("{error:?}"))?,
            )?;
            // Level 9: 0.59 × 50 × 44 = 1298, two blessings −16%: 1090.32 → 1090.
            assert_eq!(first.experience_lost.get(), 1090);
            assert_eq!(first.experience_after.get(), 5410);
            assert_eq!((first.level_before, first.level_after), (9, 8));
            assert_eq!(first.committed_character_revision.get(), 2);
            assert_eq!(first.blessings_before, ["embrace", "spark"]);
            assert!(first.blessings_after.is_empty());
            assert_eq!(
                snapshot(&harness.pool).await?,
                format!("2|2,8,5410|0|1|0|-|{}", uuid(80))
            );

            // Exact replay returns the first receipt without session authority.
            let mut reconnected = fence(1)?;
            reconnected.connection_generation =
                ConnectionGeneration::new(9).map_err(|error| format!("{error:?}"))?;
            let replay = harness
                .root
                .commit_character_death(&authority, &harness.node, reconnected, request.clone())
                .await
                .map_err(|error| format!("{error:?}"))?;
            assert_eq!(
                replay,
                CharacterDeathOutcome::AlreadyCommitted(first.clone())
            );
            // Same occurrence, changed input: conflicts even where the stored
            // outputs would match (the respawn position is not an XP input).
            let mut moved = request.clone();
            moved.respawn_position = b"temple:carlin".to_vec();
            let mut rescaled = request.clone();
            rescaled.policy.death_loss_rounding = RoundingMode::TowardZero;
            for changed in [moved, rescaled] {
                assert!(matches!(
                    harness
                        .root
                        .commit_character_death(&authority, &harness.node, fence(1)?, changed)
                        .await,
                    Err(CharacterProgressionError::ConflictingOccurrence)
                ));
            }
            assert_eq!(
                harness
                    .root
                    .reconcile_character_death(&authority, request.occurrence)
                    .await
                    .map_err(|error| format!("{error:?}"))?,
                Some(first.clone())
            );
            assert_eq!(
                harness
                    .root
                    .reconcile_character_death(
                        &authority,
                        PlayerDeathOccurrence::from_bytes(id(89))
                            .map_err(|error| format!("{error:?}"))?,
                    )
                    .await
                    .map_err(|error| format!("{error:?}"))?,
                None
            );
            // A second death cannot commit while the first respawn is pending.
            assert!(matches!(
                harness
                    .root
                    .commit_character_death(&authority, &harness.node, fence(2)?, death(81, &[])?)
                    .await,
                Err(CharacterProgressionError::RespawnPending)
            ));
            // The death cell a resumed DEATH-3 workflow reads is retained.
            let cell: (String, String, Vec<u8>, String) = sqlx::query_as(
                "SELECT death_world_id::text, death_channel_id::text, death_spatial_position, \
                        death_map_revision FROM game_character_death_receipts",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(
                cell,
                (uuid(42), uuid(43), vec![1, 2, 3, 7], "map-1".to_owned())
            );

            // Restart: the integrity check counts the death receipt.
            let restarted = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
            assert!(restarted.maintain_ready_once().await?);
            let restart_seal = harness
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let restart_authority = restarted
                .open_character_authority(&restart_seal)
                .await
                .map_err(|error| format!("restart integrity: {error:?}"))?;
            assert_eq!(
                restarted
                    .reconcile_character_death(&restart_authority, request.occurrence)
                    .await
                    .map_err(|error| format!("{error:?}"))?,
                Some(first)
            );
            let state = restarted
                .read_character_progression(
                    &restart_authority,
                    CharacterId::from_bytes(id(41)).map_err(|error| format!("{error:?}"))?,
                )
                .await
                .map_err(|error| format!("{error:?}"))?
                .ok_or("missing progression state")?;
            assert_eq!(
                (
                    state.character_revision.get(),
                    state.level,
                    state.total_experience.get()
                ),
                (2, 8, 5410)
            );
            drop(restart_authority);
            drop(restart_seal);
            drop(restarted);
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    })
}

#[test]
fn stale_fences_and_mismatched_death_intent_write_nothing() -> TestResult {
    run(|admin| {
        Box::pin(async move {
            let harness = create(admin, "death_stale", &["spark"]).await?;
            let seal = harness
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            let before = snapshot(&harness.pool).await?;
            assert_eq!(before, "1|1,9,6500|0|0|0|spark|-");

            let mut stale_connection = fence(1)?;
            stale_connection.connection_generation =
                ConnectionGeneration::new(2).map_err(|error| format!("{error:?}"))?;
            let mut stale_lease = fence(1)?;
            stale_lease.character_lease_generation = 2;
            let mut stale_scope = fence(1)?;
            stale_scope.scope_ownership_generation =
                ScopeOwnershipGeneration::new(2).map_err(|error| format!("{error:?}"))?;
            let mut other_channel = death(83, &["spark"])?;
            other_channel.death_cell.channel_id = crate::foundation::ChannelId::decode(&id(44))
                .map_err(|error| format!("{error:?}"))?;
            let cases: Vec<(
                crate::durability::character_progression::CurrentCharacterGameplayFence,
                CharacterDeathRequest<10>,
                &str,
            )> = vec![
                (
                    stale_connection,
                    death(82, &["spark"])?,
                    "AuthorityRejected",
                ),
                (stale_lease, death(82, &["spark"])?, "AuthorityRejected"),
                (stale_scope, death(82, &["spark"])?, "AuthorityRejected"),
                (
                    fence(2)?,
                    death(82, &["spark"])?,
                    "CharacterRevisionMismatch",
                ),
                (fence(1)?, other_channel, "AuthorityRejected"),
                (fence(1)?, death(84, &[])?, "HeldBlessingsMismatch"),
                (
                    fence(1)?,
                    death(85, &["embrace", "spark"])?,
                    "HeldBlessingsMismatch",
                ),
            ];
            for (stale, request, expected) in cases {
                let outcome = harness
                    .root
                    .commit_character_death(&authority, &harness.node, stale, request)
                    .await;
                assert_eq!(
                    format!("{outcome:?}"),
                    format!("Err({expected})"),
                    "unexpected outcome"
                );
            }
            let mut changed_context = death(86, &["spark"])?;
            changed_context.context.content = "content-2".into();
            changed_context.policy.context.content = "content-2".into();
            assert!(matches!(
                harness
                    .root
                    .commit_character_death(&authority, &harness.node, fence(1)?, changed_context)
                    .await,
                Err(CharacterProgressionError::ProgressionContextMismatch)
            ));
            assert_eq!(snapshot(&harness.pool).await?, before);
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    })
}

#[test]
fn mixed_chain_xp_death_stance_death_passes_integrity_and_red_gaps_fail() -> TestResult {
    run(|admin| {
        Box::pin(async move {
            let harness = create(admin, "death_mixed", &[]).await?;
            let seal = harness
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            let xp = harness
                .root
                .commit_character_experience(&authority, &harness.node, fence(1)?, award(90, 100)?)
                .await
                .map_err(|error| format!("{error:?}"))?;
            assert!(matches!(xp, ExperienceCommitOutcome::Committed(_)));
            // Level 9 at 6600 loses 1298 → 5302, level 8.
            let first = committed(
                harness
                    .root
                    .commit_character_death(&authority, &harness.node, fence(2)?, death(91, &[])?)
                    .await
                    .map_err(|error| format!("{error:?}"))?,
            )?;
            assert_eq!((first.experience_after.get(), first.level_after), (5302, 8));
            consume_pending(&harness.pool).await?;
            toggle_stance(&harness.pool, 92, 3, (8, 5302)).await?;
            // Level 8: 0.58 × 50 × 32 = 928 → 4374, still level 8.
            let second = committed(
                harness
                    .root
                    .commit_character_death(&authority, &harness.node, fence(4)?, death(93, &[])?)
                    .await
                    .map_err(|error| format!("{error:?}"))?,
            )?;
            assert_eq!(
                (
                    second.experience_before.get(),
                    second.experience_after.get()
                ),
                (5302, 4374)
            );
            assert_eq!(second.committed_character_revision.get(), 5);
            assert_eq!(
                snapshot(&harness.pool).await?,
                format!("5|5,8,4374|1|2|1|-|{}", uuid(93))
            );
            drop(authority);
            harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("mixed chain integrity: {error:?}"))?;

            // RED: before DEATH-1 the check counted XP receipts only, so the
            // chain above failed it. A gap of any kind still fails it: drop
            // the middle (stance) receipt, bypassing the guards as the test
            // owner.
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM game_character_stance_receipts")
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            assert!(
                harness.root.open_character_authority(&seal).await.is_err(),
                "a missing stance receipt must fail integrity"
            );
            drop(seal);
            harness.cleanup().await
        })
    })
}

#[test]
fn concurrent_xp_award_and_death_serialize_on_the_character_root() -> TestResult {
    run(|admin| {
        Box::pin(async move {
            let harness = create(admin, "death_race", &[]).await?;
            let other = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
            assert!(other.maintain_ready_once().await?);
            let seal = harness
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            let other_authority = other
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            let xp = harness.root.commit_character_experience(
                &authority,
                &harness.node,
                fence(1)?,
                award(94, 100)?,
            );
            let lethal = other.commit_character_death(
                &other_authority,
                &harness.node,
                fence(1)?,
                death(95, &[])?,
            );
            let (xp, lethal) = join_two(xp, lethal).await;
            let expected = match (&xp, &lethal) {
                (
                    Ok(ExperienceCommitOutcome::Committed(_)),
                    Err(CharacterProgressionError::CharacterRevisionMismatch),
                ) => "2|2,9,6600|1|0|0|-|-".to_owned(),
                (
                    Err(CharacterProgressionError::CharacterRevisionMismatch),
                    Ok(CharacterDeathOutcome::Committed(_)),
                ) => format!("2|2,8,5202|0|1|0|-|{}", uuid(95)),
                outcomes => {
                    return Err(format!("unexpected concurrent outcomes: {outcomes:?}").into());
                }
            };
            assert_eq!(snapshot(&harness.pool).await?, expected);
            drop(other_authority);
            drop(authority);
            drop(seal);
            drop(other);
            harness.cleanup().await
        })
    })
}
