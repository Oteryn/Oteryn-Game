//! PUBPROJ-B: maintenance uses actual receipt window/privacy, never a guessed cause.
use crate::bestiary_postgres_harness::{
    CHANNEL, CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, fence, id, runtime,
};
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
use crate::durability::character_death::{
    CharacterDeathRequest, DeathCell, PlayerDeathOccurrence, PublicDeathCause,
};
use crate::foundation::{ChannelId, RuntimeScopeRefV1, WorldId};
use crate::native_admission_source::public_projections::v2::{Body, Family};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
use sqlx::{Executor, Row};

fn death() -> TestResult<CharacterDeathRequest<50>> {
    let context = ProgressionRevisionContext {
        profile: "profile-1".into(),
        ruleset: "ruleset-1".into(),
        content: "content-1".into(),
        simulation: "simulation-1".into(),
        evidence: "evidence-1".into(),
        declaration: "declaration-1".into(),
    };
    let mut thresholds = [LevelThreshold {
        level: 1,
        minimum_experience: ExactI64::new(0),
    }; 50];
    for (n, t) in thresholds.iter_mut().enumerate() {
        *t = LevelThreshold {
            level: u32::try_from(n + 1)?,
            minimum_experience: ExactI64::new(i64::try_from(n)? * 20),
        };
    }
    Ok(CharacterDeathRequest {
        // Invalid native key is retained as unpublishable provenance, never PvE.
        public_cause: Some(PublicDeathCause::native_creature("Dragon", "Dragon")),
        occurrence: PlayerDeathOccurrence::from_bytes(id(180)).map_err(debug)?,
        context: context.clone(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        held_blessings: Vec::new(),
        pvp_contributors: Vec::new(),
        death_cell: DeathCell {
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(&id(WORLD)).map_err(debug)?,
                ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            ),
            spatial_position: vec![12, 13, 14, 7],
            map_revision: "death-map-1".into(),
        },
        respawn_position: b"temple:thais".to_vec(),
        policy: FiniteProgressionPolicy {
            context,
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds,
            terminal_exclusive_experience: ExactI64::new(1020),
            death_loss_numerator: 1,
            death_loss_denominator: 1,
            death_loss_rounding: RoundingMode::Floor,
        },
    })
}
async fn payload(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT payload FROM game_public_v2_heads WHERE family='character_deaths' AND subject=encode($1,'hex')::uuid::text")
        .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?)
}
// Fixture-only historical baseline. Production receipt immutability remains
// enabled during every assertion and during the real maintenance caller.
async fn history(h: &Harness, expired: bool, legacy: bool) -> TestResult {
    let mut tx = h.pool.begin().await?;
    tx.execute("SET LOCAL session_replication_role=replica")
        .await?;
    if legacy {
        sqlx::query("DELETE FROM game_public_death_classifications WHERE death_occurrence_id=encode($1,'hex')::uuid").bind(id(180).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("UPDATE game_character_death_receipts SET committed_at=(floor(extract(epoch FROM clock_timestamp()))::bigint-2592000+$2)*1000 WHERE death_occurrence_id=encode($1,'hex')::uuid")
        .bind(id(180).as_slice()).bind(if expired {-3600_i64} else {3600_i64}).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn exercise(h: &Harness, legacy: bool) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .commit_character_death(&authority, &h.node, fence(1)?, death()?)
        .await
        .map_err(debug)?;
    history(h, false, legacy).await?;
    sqlx::query("SELECT game_public_v2_deaths(encode($1,'hex')::uuid)")
        .bind(id(CHARACTER).as_slice())
        .execute(&h.pool)
        .await?;
    assert!(
        payload(h).await?.is_null(),
        "recent unknown is unavailable, not empty/PvE"
    );
    let row=sqlx::query("SELECT r.due_at_seconds,d.committed_at/1000+2592001 AS expected,floor(extract(epoch FROM clock_timestamp()))::bigint AS now FROM game_public_death_rechecks r JOIN game_character_death_receipts d USING(character_id) WHERE d.death_occurrence_id=encode($1,'hex')::uuid")
        .bind(id(180).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        row.try_get::<i64, _>("due_at_seconds")?,
        row.try_get::<i64, _>("expected")?
    );
    assert!(row.try_get::<i64, _>("due_at_seconds")? > row.try_get::<i64, _>("now")?);
    // Even an incorrectly early private deadline cannot invent known history:
    // maintenance recaptures the source window and restores its actual deadline.
    sqlx::query("UPDATE game_public_death_rechecks SET due_at_seconds=0 WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    h.root
        .public_v2_facts(&authority, Family::CharacterDeaths)
        .await
        .map_err(debug)?;
    assert!(payload(h).await?.is_null());
    let due:bool=sqlx::query_scalar("SELECT due_at_seconds>floor(extract(epoch FROM clock_timestamp()))::bigint FROM game_public_death_rechecks WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert!(due);
    // Move only the controlled fixture baseline beyond the actual30-day cut,
    // installing the deadline that its original source capture would have made.
    history(h, true, false).await?;
    sqlx::query("UPDATE game_public_death_rechecks r SET due_at_seconds=d.committed_at/1000+2592001 FROM game_character_death_receipts d WHERE d.character_id=r.character_id AND d.death_occurrence_id=encode($1,'hex')::uuid")
        .bind(id(180).as_slice()).execute(&h.pool).await?;
    h.root
        .public_v2_facts(&authority, Family::CharacterDeaths)
        .await
        .map_err(debug)?;
    assert_eq!(
        payload(h).await?,
        serde_json::json!([]),
        "only actual expiry permits the known empty recent window"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_public_death_rechecks")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    let snapshot = h
        .root
        .next_public_v2(&authority, Family::CharacterDeaths, &[])
        .await
        .map_err(debug)?
        .ok_or("expiry snapshot")?;
    assert!(matches!(&snapshot.body,Body::Deaths(rows) if rows.is_empty()));
    // Reintroduce a still-recent unknown baseline, then hide its victim. Privacy
    // permits immediate safe suppression without changing historical classification.
    history(h, false, false).await?;
    sqlx::query("SELECT game_public_v2_deaths(encode($1,'hex')::uuid)")
        .bind(id(CHARACTER).as_slice())
        .execute(&h.pool)
        .await?;
    assert!(payload(h).await?.is_null());
    let mut tx = h.pool.begin().await?;
    tx.execute("SET LOCAL session_replication_role=replica")
        .await?;
    sqlx::query(
        "UPDATE game_character_roots SET lifecycle=2 WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE game_public_death_rechecks SET due_at_seconds=0 WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    h.root
        .public_v2_facts(&authority, Family::CharacterDeaths)
        .await
        .map_err(debug)?;
    assert_eq!(payload(h).await?, serde_json::json!([]));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_public_death_rechecks")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    let classifications:Vec<bool>=sqlx::query_scalar("SELECT publishable FROM game_public_death_classifications WHERE death_occurrence_id=encode($1,'hex')::uuid").bind(id(180).as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        classifications,
        if legacy {
            Vec::<bool>::new()
        } else {
            vec![false]
        },
        "maintenance never creates or repairs a cause"
    );
    Ok(())
}
#[test]
fn pubproj_unknown_death_window_recheck_expires_only_actual_history_and_suppresses_hidden()
-> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(async move {
                    for legacy in [false, true] {
                        let started = std::time::Instant::now();
                        let h = Harness::create(
                            admin.clone(),
                            if legacy {
                                "pubproj_legacy_expiry"
                            } else {
                                "pubproj_invalid_expiry"
                            },
                            true,
                        )
                        .await?;
                        let result = exercise(&h, legacy).await;
                        h.cleanup().await?;
                        eprintln!(
                            "PUBPROJ_DEATH_EXPIRY_PG_EXECUTED legacy={legacy} elapsed_ms={}",
                            started.elapsed().as_millis()
                        );
                        result?;
                    }
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "death expiry thread failed")?
        .map_err(Into::into)
}
