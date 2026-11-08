//! PUBPROJ-D: real sealed physical occurrences, current scope authority and PG batches.
use crate::bestiary_postgres_harness::{
    CHANNEL, Harness, TestResult, WORLD, configured_admin, debug, id, publish_readiness_for_scope,
    runtime,
};
use crate::durability::public_projections::daily_kills::PhysicalKill;
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
};
use crate::foundation::{
    ChannelId, CombatDeathFixture, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
};
use sqlx::Row;

#[test]
fn pubproj_daily_kills_multichannel_batch_dedup_rollback_resync_and_gap() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(async move {
                    let start = std::time::Instant::now();
                    let h = Harness::create(admin, "pubproj_daily_kills", false).await?;
                    let result = case(&h).await;
                    h.cleanup().await?;
                    result?;
                    eprintln!(
                        "PUBPROJ_DAILY_KILLS_PG_EXECUTED elapsed_ms={}",
                        start.elapsed().as_millis()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "physical statistics thread failed")?
        .map_err(Into::into)
}
fn scope(channel: u8) -> TestResult<RuntimeScopeRefV1> {
    Ok(RuntimeScopeRefV1::channel(
        WorldId::decode(&id(WORLD)).map_err(debug)?,
        ChannelId::decode(&id(channel)).map_err(debug)?,
    ))
}
fn event(channel: u8, key: &str, boss: bool, now: i64) -> TestResult<PhysicalKill> {
    let mut owner = CombatDeathFixture::new(
        WorldId::decode(&id(WORLD)).map_err(debug)?,
        ChannelId::decode(&id(channel)).map_err(debug)?,
        ScopeOwnershipGeneration::new(1).map_err(debug)?,
    )
    .map_err(debug)?;
    owner
        .strike("physical-counter-lethal", CombatDeathFixture::HEALTH)
        .map_err(debug)?;
    let (death, _) = owner.project_death().map_err(debug)?;
    PhysicalKill::capture(death, key, boss, now)
        .ok_or_else(|| "native physical event capture".into())
}
async fn add_channel(h: &Harness, channel: u8, seed: u8) -> TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) SELECT session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,operation FROM unnest(ARRAY[1,2,3]::smallint[]) AS operation")
        .bind(id(WORLD).as_slice()).bind(id(channel).as_slice()).execute(&h.pool).await?;
    let result = h
        .writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([seed; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Assign {
                scope: scope(channel)?,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    assert!(matches!(result, AssignmentOutcome::Committed(_)));
    publish_readiness_for_scope(&h.pool, &h.root, &h.node, 1, scope(channel)?).await
}
async fn withdraw_scope(h: &Harness, scope: RuntimeScopeRefV1) -> TestResult {
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
        AdmissionAuthorityPublicationChangeV1, AdmissionPublicationPreconditionV1,
        AdmissionPublicationPurposeV1, AdmissionPublicationSourceV1,
    };
    let key = AdmissionAuthorityGuardKeyV1::Runtime(scope);
    let previous = crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(
        h.root.clone(),
    )
    .load(std::slice::from_ref(&key))
    .await
    .map_err(debug)?
    .pop()
    .flatten()
    .ok_or("current readiness")?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut state = previous.state.clone();
    let AdmissionAuthorityGuardStateV1::Runtime { ready, .. } = &mut state else {
        return Err("runtime readiness state".into());
    };
    *ready = false;
    let change = AdmissionAuthorityPublicationChangeV1 {
        key,
        source: AdmissionPublicationSourceV1 {
            authority: previous.source.authority,
            purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
            source_revision: previous.source.source_revision + 1,
            decision_identity: "pubproj-clean-stop".into(),
            source_observed_at: now,
            clock_uncertainty_seconds: 0,
        },
        precondition: AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: previous.publication_revision,
        },
        publication_revision: previous.publication_revision + 1,
        state,
    };
    let publication =
        crate::bestiary_postgres_harness::ground_native_readiness_enforcement::prepare(
            change, now,
        )?;
    let result = h
        .root
        .publish_runtime_readiness(&h.node, &publication)
        .await
        .map_err(debug)?;
    assert_eq!(
        result,
        crate::durability::admission_authority_guards::GuardPublicationDisposition::Applied
    );
    Ok(())
}
async fn case(h: &Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    add_channel(h, 44, 91).await?;
    add_channel(h, 45, 92).await?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    let rat = event(CHANNEL, "oteryn:creature.rat", false, now)?;
    // Duplicate credit/caller replay does not multiply the physical occurrence.
    h.root
        .flush_public_daily_kills(
            &authority,
            &h.node,
            scope(CHANNEL)?,
            generation,
            vec![rat.clone(), rat.clone()],
            now,
            None,
            false,
        )
        .await
        .map_err(debug)?;
    let dragon = event(44, "oteryn:creature.dragon", true, now)?;
    // Historical replay may be delivered by another current Channel: the original
    // sealed key stays unchanged and only its pre-existing fact is acknowledged.
    h.root
        .flush_public_daily_kills(
            &authority,
            &h.node,
            scope(44)?,
            generation,
            vec![rat.clone(), dragon.clone()],
            now,
            None,
            false,
        )
        .await
        .map_err(debug)?;
    assert_eq!(h.count("game_statistics_creature_deaths").await?, 2);
    let totals:(String,String)=sqlx::query_as("SELECT coalesce(sum(killed) FILTER(WHERE NOT boss),0)::text,coalesce(sum(killed) FILTER(WHERE boss),0)::text FROM game_statistics_creature_daily_kills").fetch_one(&h.pool).await?;
    assert_eq!(totals, ("1".into(), "1".into()));
    assert_eq!(
        h.count("game_statistics_creature_daily_kills").await?,
        2,
        "private per-creature dimensions remain available to ADMINAN"
    );
    let day = (now / 86400000) * 86400;
    let head: serde_json::Value = sqlx::query_scalar(
        "SELECT payload FROM game_public_v2_heads WHERE family='world_daily_kills' AND subject=$1",
    )
    .bind(day.to_string())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(head[0]["monsters_killed"], "1");
    assert_eq!(head[0]["bosses_killed"], "1");
    assert_eq!(head[0]["complete"], false);
    // Inject failure at the outbox boundary: occurrence, dimensions and head all rollback.
    sqlx::raw_sql("CREATE FUNCTION pubproj_kills_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.family='world_daily_kills' THEN RAISE EXCEPTION 'injected physical batch failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER pubproj_kills_fail BEFORE INSERT ON game_public_v2_outbox FOR EACH ROW EXECUTE FUNCTION pubproj_kills_fail()")
        .execute(&h.pool).await?;
    let cyclops = event(45, "oteryn:creature.cyclops", false, now)?;
    assert!(
        h.root
            .flush_public_daily_kills(
                &authority,
                &h.node,
                scope(45)?,
                generation,
                vec![cyclops.clone()],
                now,
                None,
                false
            )
            .await
            .is_err()
    );
    assert_eq!(h.count("game_statistics_creature_deaths").await?, 2);
    assert_eq!(h.count("game_statistics_creature_daily_kills").await?, 2);
    sqlx::raw_sql("DROP TRIGGER pubproj_kills_fail ON game_public_v2_outbox; DROP FUNCTION pubproj_kills_fail()").execute(&h.pool).await?;
    h.root
        .flush_public_daily_kills(
            &authority,
            &h.node,
            scope(45)?,
            generation,
            vec![cyclops.clone()],
            now,
            None,
            false,
        )
        .await
        .map_err(debug)?;
    let before: i64 = sqlx::query_scalar(
        "SELECT revision FROM game_public_v2_heads WHERE family='world_daily_kills' AND subject=$1",
    )
    .bind(day.to_string())
    .fetch_one(&h.pool)
    .await?;
    sqlx::query("SELECT game_public_projection_resync('world_daily_kills',false)")
        .execute(&h.pool)
        .await?;
    let after: i64 = sqlx::query_scalar(
        "SELECT revision FROM game_public_v2_heads WHERE family='world_daily_kills' AND subject=$1",
    )
    .bind(day.to_string())
    .fetch_one(&h.pool)
    .await?;
    assert!(
        after > before,
        "resync requeues retained complete replacement subjects"
    );
    // Terminal immutable-fact close may run after readiness withdrawal, but
    // it still proves the independently current node/assignment/generation.
    withdraw_scope(h, scope(45)?).await?;
    assert!(
        h.root
            .flush_public_daily_kills(
                &authority,
                &h.node,
                scope(45)?,
                generation,
                Vec::new(),
                now,
                None,
                false
            )
            .await
            .is_err()
    );
    h.root
        .flush_public_daily_kills(
            &authority,
            &h.node,
            scope(45)?,
            generation,
            Vec::new(),
            now,
            None,
            true,
        )
        .await
        .map_err(debug)?;
    // Model lost COMMIT acknowledgement: retry retained facts both at the
    // original cutoff and a later polling clock. The endpoint never advances.
    let source_before: (i64, Option<i64>, Option<i64>, bool) = sqlx::query_as(
        "SELECT cut_ms,ended_ms,gap_ms,clean FROM game_statistics_kill_sources WHERE scope_key='\\x01'::bytea||$1||$2 AND generation=1"
    ).bind(id(WORLD).as_slice()).bind(id(45).as_slice()).fetch_one(&h.pool).await?;
    let heads_before: serde_json::Value = sqlx::query_scalar(
        "SELECT coalesce(jsonb_agg(to_jsonb(h) ORDER BY world_id,subject),'[]'::jsonb) FROM game_public_v2_heads h WHERE family='world_daily_kills'"
    ).fetch_one(&h.pool).await?;
    let outbox_before = h.count("game_public_v2_outbox").await?;
    let occurrences_before = h.count("game_statistics_creature_deaths").await?;
    let dimensions_before: serde_json::Value = sqlx::query_scalar(
        "SELECT jsonb_agg(to_jsonb(d) ORDER BY world_id,day_start,creature_key,boss) FROM game_statistics_creature_daily_kills d"
    ).fetch_one(&h.pool).await?;
    for cut in [now, now + 1] {
        h.root
            .flush_public_daily_kills(
                &authority,
                &h.node,
                scope(45)?,
                generation,
                vec![cyclops.clone()],
                cut,
                None,
                true,
            )
            .await
            .map_err(debug)?;
    }
    // Same occurrence with changed immutable dimensions is also refused.
    let changed_fact = event(45, "oteryn:creature.changed", true, now)?;
    let new_occurrence = event(46, "oteryn:creature.new", false, now)?;
    for events in [vec![changed_fact], vec![new_occurrence]] {
        assert!(
            h.root
                .flush_public_daily_kills(
                    &authority,
                    &h.node,
                    scope(45)?,
                    generation,
                    events,
                    now + 1,
                    None,
                    true
                )
                .await
                .is_err()
        );
    }
    for (cut, gap, final_cut) in [
        (now - 1, None, true),
        (now, Some(now), true),
        (now, None, false),
    ] {
        assert!(
            h.root
                .flush_public_daily_kills(
                    &authority,
                    &h.node,
                    scope(45)?,
                    generation,
                    vec![cyclops.clone()],
                    cut,
                    gap,
                    final_cut
                )
                .await
                .is_err()
        );
    }
    let source_after: (i64, Option<i64>, Option<i64>, bool) = sqlx::query_as(
        "SELECT cut_ms,ended_ms,gap_ms,clean FROM game_statistics_kill_sources WHERE scope_key='\\x01'::bytea||$1||$2 AND generation=1"
    ).bind(id(WORLD).as_slice()).bind(id(45).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        source_after, source_before,
        "final retry preserves original clean endpoint"
    );
    let heads_after: serde_json::Value = sqlx::query_scalar(
        "SELECT coalesce(jsonb_agg(to_jsonb(h) ORDER BY world_id,subject),'[]'::jsonb) FROM game_public_v2_heads h WHERE family='world_daily_kills'"
    ).fetch_one(&h.pool).await?;
    assert_eq!(
        heads_after, heads_before,
        "final retries cannot increment head revisions"
    );
    assert_eq!(h.count("game_public_v2_outbox").await?, outbox_before);
    assert_eq!(
        h.count("game_statistics_creature_deaths").await?,
        occurrences_before
    );
    let dimensions_after: serde_json::Value = sqlx::query_scalar(
        "SELECT jsonb_agg(to_jsonb(d) ORDER BY world_id,day_start,creature_key,boss) FROM game_statistics_creature_daily_kills d"
    ).fetch_one(&h.pool).await?;
    assert_eq!(
        dimensions_after, dimensions_before,
        "no retry changes any counter"
    );
    let wrong_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
    assert!(
        h.root
            .flush_public_daily_kills(
                &authority,
                &h.node,
                scope(45)?,
                wrong_generation,
                Vec::new(),
                now,
                None,
                true
            )
            .await
            .is_err()
    );
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(scope(CHANNEL)?)
        .await
        .map_err(debug)?
        .ok_or("current scope predecessor")?;
    let outcome = h
        .writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([93; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Revoke {
                scope: scope(CHANNEL)?,
                predecessor,
            },
        })
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, AssignmentOutcome::Committed(_)));
    let row=sqlx::query("SELECT cut_ms,gap_ms,clean FROM game_statistics_kill_sources WHERE scope_key='\\x01'::bytea||$1||$2 AND generation=1")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        row.try_get::<Option<i64>, _>("gap_ms")?,
        Some(row.try_get("cut_ms")?)
    );
    assert!(!row.try_get::<bool, _>("clean")?);
    let pending: Option<i64> =
        sqlx::query_scalar("SELECT game_public_v2_source_pending('world_daily_kills')")
            .fetch_one(&h.pool)
            .await?;
    assert!(
        pending.is_some_and(|cut| cut <= now),
        "unclean source closure cannot become fresh/complete from silence"
    );
    // Historical payload fixture models a retained pre-outage aggregate; it is
    // never used as current Node/scope authority. Real maintenance must erase
    // payloads while preserving the non-payload subject fence and tombstone.
    let old_day = day - 401 * 86400;
    sqlx::query("INSERT INTO game_statistics_kill_days(world_id,day_start) VALUES(encode($1,'hex')::uuid,$2)").bind(id(WORLD).as_slice()).bind(old_day).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_statistics_creature_daily_kills VALUES(encode($1,'hex')::uuid,$2,'oteryn:creature.rat',false,7)").bind(id(WORLD).as_slice()).bind(old_day).execute(&h.pool).await?;
    h.root
        .maintain_public_daily_kills(&authority)
        .await
        .map_err(debug)?;
    let erased:bool=sqlx::query_scalar("SELECT erased FROM game_statistics_kill_days WHERE world_id=encode($1,'hex')::uuid AND day_start=$2").bind(id(WORLD).as_slice()).bind(old_day).fetch_one(&h.pool).await?;
    assert!(erased);
    let old_payload:serde_json::Value=sqlx::query_scalar("SELECT payload FROM game_public_v2_heads WHERE family='world_daily_kills' AND world_id=encode($1,'hex')::uuid::text AND subject=$2").bind(id(WORLD).as_slice()).bind(old_day.to_string()).fetch_one(&h.pool).await?;
    assert_eq!(old_payload, serde_json::json!([]));
    let retained:i64=sqlx::query_scalar("SELECT count(*) FROM game_statistics_creature_daily_kills WHERE world_id=encode($1,'hex')::uuid AND day_start=$2").bind(id(WORLD).as_slice()).bind(old_day).fetch_one(&h.pool).await?;
    assert_eq!(retained, 0);
    Ok(())
}
