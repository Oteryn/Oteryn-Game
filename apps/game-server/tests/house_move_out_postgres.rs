#![cfg(any())] // CP slot: enable with native module registration.
#![allow(clippy::expect_used)]
// Native private-root target, matching character_authority_postgres.rs. Registration of the
// owned disposition modules remains a CP slot; qualification uses an exact native-source overlay.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;
pub use production_server::admission_evidence;
pub use production_server::domain;
#[allow(dead_code, unused_imports)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;
#[allow(dead_code, unused_imports)]
#[path = "../src/premium/mod.rs"]
mod premium;

// QUALIFICATION_CASES_BEGIN: exact cases also run in the native private-root library.
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, WORLD, configured_admin, debug, id, item_fence,
    runtime, scope,
};
use crate::durability::house_move_out::{
    HouseMoveOutError as Error, HouseMoveOutOutcome as Outcome, HouseMoveOutPass as Pass,
    MoveOutCancellation, MoveOutNotice, PRIVATE_DISPOSITION_SCHEMA,
};
use crate::durability::house_scope_handoff::{
    HouseCommitOutcome, HouseEntryCommit, HouseEntryRequest, HouseHandoffDirection, HouseId,
    HousePrepareOutcome, HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome,
    HouseScopeAssignmentRequest,
};
use crate::durability::house_transfer::PRIVATE_DISPOSITION_TRANSFER_SCHEMA;
use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
use crate::foundation::{CommandId, CommandRef, GameSessionId, WorldId};
use sqlx::Executor;
use std::{future::Future, pin::pin, task::Poll};
const HOUSE: &str = "oteryn:content.house.thais_1";
const OP: u8 = 70;
fn house() -> TestResult<HouseId> {
    Ok(HouseId::new(WorldId::decode(&id(WORLD)).map_err(debug)?, HOUSE).ok_or("house")?)
}
fn command(n: u64) -> TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&id(SESSION)).map_err(debug)?,
        CommandId::new(n).map_err(debug)?,
    ))
}
fn notice(days: u8) -> TestResult<Pass> {
    Ok(Pass::Notice {
        fence: item_fence()?,
        command: command(100)?,
        request: MoveOutNotice {
            operation: id(OP),
            house: house()?,
            ownership_revision: 1,
            notice_days: days,
        },
    })
}
fn freeze() -> TestResult<Pass> {
    Ok(Pass::Freeze {
        operation: id(OP),
        house: house()?,
        execution_scope: scope()?,
        scope_generation: 1,
    })
}
fn step(n: u64) -> TestResult<Pass> {
    Ok(Pass::Step {
        operation: id(OP),
        house: house()?,
        execution_scope: scope()?,
        scope_generation: 1,
        step: n,
    })
}
fn cancellation(n: u64) -> TestResult<Pass> {
    Ok(Pass::Cancel {
        fence: item_fence()?,
        command: command(n)?,
        request: MoveOutCancellation {
            operation: id(OP),
            house: house()?,
            ownership_revision: 1,
        },
    })
}
async fn race_passes(
    h: &Harness,
    a: Pass,
    b: Pass,
) -> TestResult<(Result<Outcome, Error>, Result<Outcome, Error>)> {
    // Two genuine ready native Roots model two World workers; no racing authority setup or
    // dependence on one Root's opportunistic single idle holder. Both seals are really checked.
    let second = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
    assert!(second.maintain_ready_once().await?);
    let seal = h.recovery.seal_current().map_err(debug)?;
    let first_auth = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let second_auth = second
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    Ok(join(
        h.root.run_private_house_move_out(&first_auth, &h.node, a),
        second.run_private_house_move_out(&second_auth, &h.node, b),
    )
    .await)
}
async fn apply(h: &Harness, p: Pass) -> Result<Outcome, Error> {
    let seal = h
        .recovery
        .seal_current()
        .map_err(|_| Error::AuthorityRejected)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(|error| {
            eprintln!("native authority open failed: {error:?}");
            Error::AuthorityRejected
        })?;
    h.root
        .run_private_house_move_out(&authority, &h.node, p)
        .await
}
async fn join<A: Future, B: Future>(a: A, b: B) -> (A::Output, B::Output) {
    let mut a = pin!(a);
    let mut b = pin!(b);
    let mut av = None;
    let mut bv = None;
    std::future::poll_fn(|cx| {
        if av.is_none()
            && let Poll::Ready(v) = a.as_mut().poll(cx)
        {
            av = Some(v);
        }
        if bv.is_none()
            && let Poll::Ready(v) = b.as_mut().poll(cx)
        {
            bv = Some(v);
        }
        match (av.take(), bv.take()) {
            (Some(a), Some(b)) => Poll::Ready((a, b)),
            (a, b) => {
                av = a;
                bv = b;
                Poll::Pending
            }
        }
    })
    .await
}
fn uuid(bytes: [u8; 16]) -> String {
    let h: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}
async fn bootstrap_second_subject(h: &Harness) -> TestResult<[u8; 16]> {
    use crate::durability::native_admission_source::{
        DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
        SourceObservation,
    };
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let provenance = FreshStoreProvenance {
        namespace: "store:house-custody-fixture".into(),
        authorization: "owner:approved-local-fixture".into(),
        source_authority: "platform".into(),
        initialized_at: 10,
    };
    let descriptor = DescriptorRegistration {
        revision: 1,
        facts: vec![1],
        installed_at: 10,
    };
    assert!(
        h.root
            .record_native_source_descriptor_issuance(
                "platform",
                descriptor.clone(),
                Some(provenance.clone())
            )
            .await
            .map_err(debug)?
    );
    h.root
        .initialize_native_admission_source(&h.node, provenance, descriptor)
        .await
        .map_err(debug)?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let account = uuid(id(82));
    // Authenticated source-observation fixture through the real native S2 owner, not an
    // authority flag in the disposition API. Platform TLS/producer delivery is not qualified.
    let observation = format!(
        r#"{{"version":1,"operation":"ReadAccountSecurityV1","result":"observed","source_authority":"platform","source_revision":"1","decision_identity":"1","source_observed_at":"{now}","clock_uncertainty_seconds":"0","account_id":"{account}","purpose":"platform_security","scope":"fresh_admission","allowed":true,"minimum_valid_generation":"1"}}"#
    );
    h.root
        .accept_native_source_observation(
            &h.node,
            SourceObservation {
                source_authority: "platform".into(),
                operation: NativeSourceOperation::ReadAccountSecurityV1,
                subject: NativeSourceSubject::account_security(account.clone()).map_err(debug)?,
                source_revision: 1,
                decision_identity: "1".into(),
                observed_at: now,
                semantic_facts: observation.into_bytes(),
            },
        )
        .await
        .map_err(debug)?;
    let operation = uuid(id(83));
    let decision = uuid(id(84));
    let world = uuid(id(WORLD));
    let expires = now + 120;
    let wire = format!(
        r#"{{"contract_version":2,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"{decision}","source_revision":"2","operation_id":"{operation}","operation":"INITIAL_CHARACTER_BOOTSTRAP","account_id":"{account}","target_world_id":"{world}","requested_name":"Second Subject","interpretation_context":{{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"}},"issued_at_source":"{now}","expires_at_source":"{expires}","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}}"#
    );
    let intent =
        crate::character_bootstrap_intent::decode_producer_response(wire.as_bytes(), id(83))
            .map_err(debug)?;
    let record = h
        .root
        .bootstrap_character(&authority, &h.node, &intent)
        .await
        .map_err(debug)?;
    Ok(*record.character_id.as_bytes())
}
async fn setup(h: &Harness, items: usize) -> TestResult {
    sqlx::raw_sql(PRIVATE_DISPOSITION_SCHEMA)
        .execute(&h.pool)
        .await?;
    sqlx::raw_sql(PRIVATE_DISPOSITION_TRANSFER_SCHEMA)
        .execute(&h.pool)
        .await?;
    let second = bootstrap_second_subject(h).await?;
    // Only preexisting item/ownership state is authored with replication triggers disabled.
    // All native custody/provenance/Inbox guards are active for candidate operations.
    let mut tx = h.pool.begin().await?;
    tx.execute("SET LOCAL session_replication_role=replica")
        .await?;
    sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)").bind(id(WORLD).as_slice()).bind(HOUSE).bind(id(CHARACTER).as_slice()).bind(id(90).as_slice()).execute(&mut *tx).await?;
    for n in 0..items {
        let mut it = id(100);
        it[14] = (n >> 8) as u8;
        it[15] = n as u8;
        let mut placement = id(220);
        placement[14] = (n >> 8) as u8;
        placement[15] = n as u8;
        let subject = if n % 2 == 0 { id(CHARACTER) } else { second };
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:content.item.vase','r1',1,1,encode($3,'hex')::uuid)").bind(it.as_slice()).bind(id(WORLD).as_slice()).bind(placement.as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_interior_locations(item_instance_id,world_id,house_key,spatial_position,stack_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,encode($6,'hex')::uuid)").bind(it.as_slice()).bind(id(WORLD).as_slice()).bind(HOUSE).bind([1_u8,2,3].as_slice()).bind((n+1) as i64).bind(placement.as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)").bind(it.as_slice()).bind(id(WORLD).as_slice()).bind(HOUSE).bind(subject.as_slice()).bind(placement.as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
// Initial retained private NOTICE from an earlier day. It is fixture history only: current
// authority is independently re-read by the actual Root job, never reconstructed from it.
async fn past_notice(h: &Harness) -> TestResult {
    sqlx::query("INSERT INTO game_house_dispositions(operation_id,world_id,house_key,owner_character_id,ownership_revision,game_session_id,command_id,command_binding,notice_days,requested_at_ms,due_at_ms) SELECT encode($1,'hex')::uuid,world_id,house_key,owner_character_id,ownership_revision,encode($2,'hex')::uuid,90,$3,1,n-172800000,n-86400000 FROM game_house_ownership CROSS JOIN (SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS n) clock WHERE house_key=$4")
  .bind(id(OP).as_slice()).bind(id(SESSION).as_slice()).bind([1_u8].as_slice()).bind(HOUSE).execute(&h.pool).await?;
    Ok(())
}
fn run(tag: &'static str, case: impl AsyncFnOnce(&Harness) -> TestResult) -> TestResult {
    let admin =
        configured_admin().ok_or("PostgreSQL17.6 admin route required; no skipped success")?;
    runtime()?.block_on(async {
        let h = Harness::create(admin, tag, false).await?;
        let result = case(&h).await;
        h.cleanup().await?;
        result
    })
}
async fn counts(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok((
        h.count("game_item_house_interior_locations").await?,
        h.count("game_item_house_reclaim_provenance").await?,
        h.count("game_item_character_inbox_locations").await?,
        h.count("game_house_disposition_steps").await?,
    ))
}

async fn reconstruct_root(h: &mut Harness) -> TestResult {
    use crate::durability::runtime_scope_assignment::RuntimeScopeAssignmentWriter;

    // Reconstruct both holders of the old Root. Readiness/assignment-slot restoration may
    // overlap; no resumed disposition pass runs until every old Root clone is gone.
    let old_liveness = h.root.root_liveness();
    let root = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
    assert_ne!(root.root_identity(), h.root.root_identity());
    assert!(root.maintain_ready_once().await?);
    let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "bestiary-writer")
        .await
        .map_err(debug)?;
    assert!(writer.unreconciled().is_none());
    drop(std::mem::replace(&mut h.writer, writer));
    drop(std::mem::replace(&mut h.root, root));
    assert!(old_liveness.upgrade().is_none());
    // apply() opens a new reconciled authority from the actual retained recovery seal.
    // PostgreSQL, the recovery store and Node incarnation are not restarted or replaced.
    Ok(())
}

async fn retained_notice_identity(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT (to_jsonb(o)-ARRAY['state','frozen_count','frozen_digest','next_step'])::text \
         FROM game_house_dispositions o WHERE operation_id=encode($1,'hex')::uuid",
    )
    .bind(id(OP).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

#[test]
fn frozen_native_item_and_provenance_mutations_share_the_content_fence() -> TestResult {
    run("frozen_native_sources", async |h| {
        setup(h, 1).await?;
        let abi: i32 = sqlx::query_scalar("SELECT game_house_disposition_guild_fence_abi()")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(abi, 1);
        let mut runtime_reader = h.pool.begin().await?;
        runtime_reader
            .execute("SET LOCAL ROLE oteryn_game_runtime")
            .await?;
        let runtime_abi: i32 =
            sqlx::query_scalar("SELECT game_house_disposition_guild_fence_abi()")
                .fetch_one(&mut *runtime_reader)
                .await?;
        assert_eq!(runtime_abi, 1);
        let mutation_authority: bool = sqlx::query_scalar(
            "SELECT has_function_privilege(current_user, \
             'game_house_disposition_fence_check(uuid,text,uuid,boolean)', 'EXECUTE')",
        )
        .fetch_one(&mut *runtime_reader)
        .await?;
        assert!(!mutation_authority);
        runtime_reader.rollback().await?;
        // A native row trigger must never wait for an anchor held by a disposition,
        // which may itself need that row. The entire rejected statement rolls back.
        let mut owner = h.pool.begin().await?;
        sqlx::query("SELECT 1 FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR UPDATE")
            .bind(id(WORLD).as_slice()).bind(HOUSE).execute(&mut *owner).await?;
        for statement in [
            "UPDATE game_item_instances SET quantity=quantity WHERE item_instance_id IN(SELECT item_instance_id FROM game_item_house_interior_locations)",
            "DELETE FROM game_item_house_reclaim_provenance",
        ] {
            let result = sqlx::query(statement).execute(&h.pool).await;
            let observed = match &result {
                Ok(done) => format!("accepted; rows_affected={}", done.rows_affected()),
                Err(sqlx::Error::Database(error)) => {
                    format!("SQLSTATE={:?}; message={}", error.code(), error.message())
                }
                Err(error) => format!("non-database SQLx error: {error}"),
            };
            assert!(
                matches!(result, Err(sqlx::Error::Database(ref error)) if error.code().as_deref() == Some("55P03")),
                "native mutation must refuse the held ownership anchor with55P03: {statement}; {observed}"
            );
        }
        owner.rollback().await?;
        past_notice(h).await?;
        apply(h, freeze()?).await?;
        for statement in [
            "UPDATE game_item_instances SET quantity=quantity WHERE item_instance_id IN(SELECT item_instance_id FROM game_item_house_interior_locations)",
            "DELETE FROM game_item_instances WHERE item_instance_id IN(SELECT item_instance_id FROM game_item_house_interior_locations)",
            "DELETE FROM game_item_house_reclaim_provenance",
        ] {
            let result = sqlx::query(statement).execute(&h.pool).await;
            let Err(sqlx::Error::Database(error)) = result else {
                return Err("frozen native source mutation was not refused by the fence".into());
            };
            assert_eq!(error.code().as_deref(), Some("23514"));
            assert!(error.message().contains("house disposition content fence"));
        }
        assert_eq!(counts(h).await?, (1, 1, 0, 0));
        // The real custody path stamps its census before retiring provenance/location;
        // the new guards must still admit that exact same-transaction native delivery.
        assert!(matches!(apply(h, step(0)?).await?, Outcome::Step { view, .. } if view.drained));
        assert_eq!(counts(h).await?, (0, 0, 1, 1));
        Ok(())
    })
}

#[test]
fn ordinary_drained_fence_remains_without_a_guild_owner_installation() -> TestResult {
    run("ordinary_drained_fence", async |h| {
        setup(h, 0).await?;
        let guild_owner_absent: bool = sqlx::query_scalar(
            "SELECT to_regclass(format('%I.game_house_guild_dispositions',current_schema())) IS NULL \
             AND to_regprocedure(format('%I.game_house_guild_disposition_fence_check(uuid,text,uuid,boolean)',current_schema())) IS NULL",
        )
        .fetch_one(&h.pool)
        .await?;
        assert!(guild_owner_absent);
        // No typed Guild schema or fake hook is installed in the ordinary native fixture.
        sqlx::query(
            "SELECT game_house_disposition_fence_check(encode($1,'hex')::uuid,$2,NULL,false)",
        )
        .bind(id(WORLD).as_slice())
        .bind(HOUSE)
        .execute(&h.pool)
        .await?;
        past_notice(h).await?;
        apply(h, freeze()?).await?;
        let drained = apply(h, step(0)?).await?;
        assert!(matches!(drained, Outcome::Step { view, .. } if view.drained));
        // Both actual native trigger entry points share this boundary. Ordinary state3 is
        // still active even though no items remain and no Guild installation is present.
        for deleting in [false, true] {
            let rejected = sqlx::query(
                "SELECT game_house_disposition_fence_check(encode($1,'hex')::uuid,$2,NULL,$3)",
            )
            .bind(id(WORLD).as_slice())
            .bind(HOUSE)
            .bind(deleting)
            .execute(&h.pool)
            .await;
            assert!(
                matches!(rejected, Err(sqlx::Error::Database(ref error)) if error.code().as_deref() == Some("23514"))
            );
        }
        assert_eq!(counts(h).await?, (0, 0, 0, 1));
        assert!(
            matches!(apply(h, step(0)?).await?, Outcome::Step { view, replayed: true, .. } if view.drained)
        );
        Ok(())
    })
}

#[test]
fn notice_is_current_fenced_bounded_and_idempotent() -> TestResult {
    run("hm_notice", async |h| {
        setup(h, 0).await?;
        assert!(matches!(
            apply(h, notice(0)?).await,
            Err(Error::InvalidInput)
        ));
        let Pass::Notice {
            mut fence,
            command,
            request,
        } = notice(1)?
        else {
            return Err("notice".into());
        };
        fence.character_lease_generation = 2;
        assert!(matches!(
            apply(
                h,
                Pass::Notice {
                    fence,
                    command,
                    request
                }
            )
            .await,
            Err(Error::AuthorityRejected)
        ));
        assert_eq!(h.count("game_house_dispositions").await?, 0);
        let first = apply(h, notice(1)?).await.map_err(debug)?;
        let Pass::Notice {
            mut fence,
            command: stored_command,
            request,
        } = notice(1)?
        else {
            return Err("notice".into());
        };
        fence.game_session_id = GameSessionId::decode(&id(59)).map_err(debug)?;
        assert!(matches!(
            apply(
                h,
                Pass::Notice {
                    fence,
                    command: stored_command,
                    request
                }
            )
            .await,
            Err(Error::AuthorityRejected)
        ));

        assert_eq!(apply(h, notice(1)?).await.map_err(debug)?, first);
        assert!(matches!(
            apply(h, notice(2)?).await,
            Err(Error::ConflictingOperation)
        ));
        let Pass::Notice {
            fence,
            command,
            mut request,
        } = notice(1)?
        else {
            return Err("notice".into());
        };
        request.operation = id(71);
        assert!(matches!(
            apply(
                h,
                Pass::Notice {
                    fence,
                    command,
                    request
                }
            )
            .await,
            Err(Error::ConflictingOperation)
        ));
        assert!(matches!(apply(h, freeze()?).await, Err(Error::NotDue)));
        let span: i64 =
            sqlx::query_scalar("SELECT due_at_ms-requested_at_ms FROM game_house_dispositions")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(span, 86_400_000);
        let err = sqlx::query("UPDATE game_house_dispositions SET cause='EVICTION'")
            .execute(&h.pool)
            .await;
        assert!(err.is_err());
        Ok(())
    })
}

#[test]
fn unloaded_house_101_items_return_to_real_reclaim_subjects_once() -> TestResult {
    run("hm_101", async |h| {
        setup(h, 101).await?;
        past_notice(h).await?;
        let assigned: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_runtime_scope_assignments WHERE scope_kind=2",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(assigned, 0);
        let frozen = apply(h, freeze()?).await.map_err(debug)?;
        let Outcome::Frozen(v) = frozen else {
            return Err("freeze".into());
        };
        assert_eq!(v.frozen_count, Some(101));
        let Outcome::Step {
            items: first,
            view: v,
            replayed,
        } = apply(h, step(0)?).await.map_err(debug)?
        else {
            return Err("step".into());
        };
        assert_eq!(first.len(), 100);
        assert!(!replayed);
        assert!(!v.drained);
        assert_eq!(v.next_step, 1);
        let Outcome::Step {
            items: last,
            view: v,
            ..
        } = apply(h, step(1)?).await.map_err(debug)?
        else {
            return Err("step".into());
        };
        assert_eq!(last.len(), 1);
        assert!(v.drained);
        assert_eq!(counts(h).await?, (0, 0, 101, 2));
        let mismatch:i64=sqlx::query_scalar("SELECT count(*) FROM game_house_disposition_items c JOIN game_item_character_inbox_locations l USING(item_instance_id) WHERE l.character_id<>(c.source_provenance->>'reclaim_subject_character_id')::uuid OR l.ordinal<>c.ordinal").fetch_one(&h.pool).await?;
        assert_eq!(mismatch, 0);
        let total: String =
            sqlx::query_scalar("SELECT sum(committed)::text FROM game_character_inbox_counters")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(total, "101");
        let Pass::Step {
            operation,
            house,
            execution_scope,
            step: old_step,
            ..
        } = step(0)?
        else {
            return Err("step".into());
        };
        let Outcome::Step {
            items, replayed, ..
        } = apply(
            h,
            Pass::Step {
                operation,
                house,
                execution_scope,
                scope_generation: 99,
                step: old_step,
            },
        )
        .await
        .map_err(debug)?
        else {
            return Err("replay".into());
        };
        assert!(replayed);
        assert_eq!(items, first);
        assert_eq!(counts(h).await?, (0, 0, 101, 2));
        assert!(matches!(
            apply(h, step(2)?).await,
            Err(Error::UnexpectedStep)
        ));
        // DRAINED is custody completion only: native ownership and the content fence remain.
        assert_eq!(h.count("game_house_ownership").await?, 1);
        assert!(
            sqlx::query("DELETE FROM game_item_house_interior_locations WHERE false")
                .execute(&h.pool)
                .await
                .is_ok()
        );
        Ok(())
    })
}

#[test]
fn reconstructed_roots_resume_frozen_and_partial_custody_without_redelivery() -> TestResult {
    let admin =
        configured_admin().ok_or("PostgreSQL17.6 admin route required; no skipped success")?;
    runtime()?.block_on(async {
        let mut h = Harness::create(admin, "hm_reopen", false).await?;
        let result: TestResult = async {
            setup(&h, 101).await?;
            // Retained earlier-day NOTICE fixture, not a substitute for a current admission.
            past_notice(&h).await?;
            let identity = retained_notice_identity(&h).await?;
            let frozen = apply(&h, freeze()?).await.map_err(debug)?;
            let Outcome::Frozen(manifest) = &frozen else {
                return Err("freeze".into());
            };
            assert_eq!(manifest.operation, id(OP));
            assert_eq!(manifest.frozen_count, Some(101));

            reconstruct_root(&mut h).await?;
            assert_eq!(apply(&h, freeze()?).await.map_err(debug)?, frozen);
            assert_eq!(retained_notice_identity(&h).await?, identity);
            // A retained operation is not current World-job authority for a new step.
            let Pass::Step {
                operation, house, execution_scope, step: next, ..
            } = step(0)? else {
                return Err("step".into());
            };
            assert!(matches!(
                apply(&h, Pass::Step {
                    operation, house, execution_scope, scope_generation: 99, step: next,
                }).await,
                Err(Error::AuthorityRejected)
            ));
            assert_eq!(counts(&h).await?, (101, 101, 0, 0));
            let Outcome::Step { items: first, view, replayed } =
                apply(&h, step(0)?).await.map_err(debug)? else {
                    return Err("first step".into());
                };
            assert!(!replayed);
            assert_eq!(first.len(), 100);
            assert_eq!(view.next_step, 1);
            assert!(!view.drained);
            assert_eq!(counts(&h).await?, (1, 1, 100, 1));

            reconstruct_root(&mut h).await?;
            let Outcome::Step { items, view: resumed, replayed } =
                apply(&h, step(0)?).await.map_err(debug)? else {
                    return Err("partial replay".into());
                };
            assert!(replayed);
            assert_eq!(items, first);
            assert_eq!(resumed, view);
            assert_eq!(counts(&h).await?, (1, 1, 100, 1));
            let Outcome::Step { items: last, view: drained, replayed } =
                apply(&h, step(1)?).await.map_err(debug)? else {
                    return Err("last step".into());
                };
            assert!(!replayed);
            assert_eq!(last.len(), 1);
            assert!(drained.drained);
            assert_eq!(drained.next_step, 2);
            assert_eq!(drained.frozen_count, manifest.frozen_count);
            assert_eq!(drained.frozen_digest, manifest.frozen_digest);

            reconstruct_root(&mut h).await?;
            for (index, expected) in [(0, first), (1, last)] {
                let Outcome::Step { items, view, replayed } =
                    apply(&h, step(index)?).await.map_err(debug)? else {
                        return Err("terminal replay".into());
                    };
                assert!(replayed);
                assert_eq!(items, expected);
                assert_eq!(view, drained);
            }
            assert!(matches!(apply(&h, step(2)?).await, Err(Error::UnexpectedStep)));
            assert_eq!(retained_notice_identity(&h).await?, identity);
            assert_eq!(counts(&h).await?, (0, 0, 101, 2));
            assert_eq!(h.count("game_character_inbox_deliveries").await?, 101);
            let mismatched: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_house_disposition_items c \
                 LEFT JOIN game_item_character_inbox_locations l USING(item_instance_id) \
                 WHERE c.operation_id=encode($1,'hex')::uuid AND \
                 (l.character_id IS DISTINCT FROM (c.source_provenance->>'reclaim_subject_character_id')::uuid \
                  OR l.ordinal IS DISTINCT FROM c.ordinal)",
            ).bind(id(OP).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(mismatched, 0);
            let recipients: (i64, i64, i64) = sqlx::query_as(
                "SELECT count(*),min(n),max(n) FROM \
                 (SELECT count(*) AS n FROM game_item_character_inbox_locations GROUP BY character_id) subjects",
            ).fetch_one(&h.pool).await?;
            assert_eq!(recipients, (2, 50, 51));
            let total: String = sqlx::query_scalar(
                "SELECT sum(committed)::text FROM game_character_inbox_counters",
            ).fetch_one(&h.pool).await?;
            assert_eq!(total, "101");
            // Custody completion retains the actual0038 owner/revision and content fence.
            // This is not the unmerged HOUSE-1a property/Account-slot release transaction.
            let owner: (Vec<u8>, String) = sqlx::query_as(
                "SELECT uuid_send(owner_character_id),ownership_revision::text \
                 FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2",
            ).bind(id(WORLD).as_slice()).bind(HOUSE).fetch_one(&h.pool).await?;
            assert_eq!(owner, (id(CHARACTER).to_vec(), "1".into()));
            let refusal = sqlx::query(
                "SELECT game_house_disposition_fence_check(encode($1,'hex')::uuid,$2,NULL,false)",
            ).bind(id(WORLD).as_slice()).bind(HOUSE).execute(&h.pool).await;
            let Err(sqlx::Error::Database(refusal)) = refusal else {
                return Err("drained content fence must remain active".into());
            };
            assert_eq!(refusal.code().as_deref(), Some("23514"));
            assert!(refusal.message().contains("house disposition content fence"));
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}

#[test]
fn native_inbox_failure_rolls_back_source_provenance_cursor_and_evidence() -> TestResult {
    run("hm_rollback", async |h| {
        setup(h, 3).await?;
        past_notice(h).await?;
        apply(h, freeze()?).await.map_err(debug)?;
        sqlx::query(
            "DELETE FROM game_character_inbox_cause_kinds WHERE cause_kind='HOUSE_DISPOSITION'",
        )
        .execute(&h.pool)
        .await?;
        let result = apply(h, step(0)?).await;
        assert!(result.is_err());
        assert_eq!(counts(h).await?, (3, 3, 0, 0));
        assert_eq!(h.count("game_character_inbox_deliveries").await?, 0);
        let marked: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_house_disposition_items WHERE moved_step IS NOT NULL",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(marked, 0);
        sqlx::query("INSERT INTO game_character_inbox_cause_kinds VALUES('HOUSE_DISPOSITION')")
            .execute(&h.pool)
            .await?;
        let Outcome::Step { items, view, .. } = apply(h, step(0)?).await.map_err(debug)? else {
            return Err("retry".into());
        };
        assert_eq!(items.len(), 3);
        assert!(view.drained);
        assert_eq!(counts(h).await?, (0, 0, 3, 1));
        Ok(())
    })
}

#[test]
fn native_source_and_frozen_manifest_cannot_be_forged_or_edited() -> TestResult {
    run("hm_guards", async |h| {
        setup(h, 2).await?;
        past_notice(h).await?;
        apply(h, freeze()?).await.map_err(debug)?;
        assert!(
            sqlx::query(
                "UPDATE game_item_house_interior_locations SET stack_ordinal=stack_ordinal+10"
            )
            .execute(&h.pool)
            .await
            .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM game_item_house_interior_locations")
                .execute(&h.pool)
                .await
                .is_err()
        );
        assert!(sqlx::query("UPDATE game_house_disposition_items SET source_provenance=jsonb_set(source_provenance,'{reclaim_subject_character_id}',to_jsonb('00000000-0000-7000-8000-000000000000'::text))").execute(&h.pool).await.is_err());
        // A self-issued moving flag cannot commit: native Inbox delivery and step proof are required.
        let mut tx = h.pool.begin().await?;
        sqlx::query("UPDATE game_house_disposition_items SET moved_step=0,moving_xact_id=pg_current_xact_id()").execute(&mut *tx).await?;
        assert!(tx.commit().await.is_err());
        assert_eq!(counts(h).await?, (2, 2, 0, 0));
        // Use a pinned connection: pool SET ROLE would otherwise be a different session.
        let mut connection = h.pool.acquire().await?;
        sqlx::query("SET ROLE oteryn_game_runtime")
            .execute(&mut *connection)
            .await?;
        assert!(
            sqlx::query("SELECT * FROM game_house_disposition_deliver(NULL,0,NULL)")
                .fetch_all(&mut *connection)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM game_item_house_interior_locations")
                .execute(&mut *connection)
                .await
                .is_err()
        );
        sqlx::query("RESET ROLE").execute(&mut *connection).await?;
        Ok(())
    })
}

#[test]
fn current_job_scope_owner_and_native_ownership_revision_are_independent() -> TestResult {
    run("hm_fences", async |h| {
        setup(h, 1).await?;
        past_notice(h).await?;
        let Pass::Freeze {
            operation,
            house,
            execution_scope,
            ..
        } = freeze()?
        else {
            return Err("freeze".into());
        };
        assert!(matches!(
            apply(
                h,
                Pass::Freeze {
                    operation,
                    house,
                    execution_scope,
                    scope_generation: 2
                }
            )
            .await,
            Err(Error::AuthorityRejected)
        ));
        apply(h, freeze()?).await.map_err(debug)?;
        sqlx::query("UPDATE game_house_ownership SET ownership_revision=2,acquisition_receipt_ref=encode($1,'hex')::uuid").bind(id(91).as_slice()).execute(&h.pool).await?;
        assert!(matches!(
            apply(h, step(0)?).await,
            Err(Error::OwnershipChanged)
        ));
        assert_eq!(counts(h).await?, (1, 1, 0, 0));
        Ok(())
    })
}

#[test]
fn concurrent_same_step_and_new_notice_serialize_without_double_delivery() -> TestResult {
    run("hm_race", async |h| {
        setup(h, 1).await?;
        past_notice(h).await?;
        apply(h, freeze()?).await.map_err(debug)?;
        let (a, b) = race_passes(h, step(0)?, step(0)?).await?;
        let a = a.map_err(debug)?;
        let b = b.map_err(debug)?;
        let (Outcome::Step { replayed: ar, .. }, Outcome::Step { replayed: br, .. }) = (a, b)
        else {
            return Err("step outcomes".into());
        };
        assert_ne!(ar, br);
        assert_eq!(counts(h).await?, (0, 0, 1, 1));
        let Pass::Notice { fence, request, .. } = notice(1)? else {
            return Err("notice".into());
        };
        let mut request = request;
        request.operation = id(71);
        assert!(matches!(
            apply(
                h,
                Pass::Notice {
                    fence,
                    command: command(101)?,
                    request
                }
            )
            .await,
            Err(Error::ConflictingOperation)
        ));
        Ok(())
    })
}

fn entry_request(handoff: u8) -> TestResult<HouseEntryRequest> {
    Ok(HouseEntryRequest {
        direction: HouseHandoffDirection::Entry,
        handoff_id: id(handoff),
        character_id: crate::foundation::CharacterId::decode(&id(CHARACTER)).map_err(debug)?,
        source_game_session_id: GameSessionId::decode(&id(SESSION)).map_err(debug)?,
        source_connection_generation: 1,
        source_character_lease_generation: 1,
        source_scope: scope()?,
        source_scope_ownership_generation: 1,
        house: house()?,
        house_scope_ownership_generation: 1,
        acl_revision: 0,
        guild_revisions: None,
        reserved_tile: vec![1, 2, 3],
        prepared_at: 100,
    })
}
async fn prepare_entry(h: &Harness) -> TestResult {
    sqlx::query(
        "INSERT INTO game_durability_session_use_ledgers VALUES(encode($1,'hex')::uuid,1,true,1,1)",
    )
    .bind(id(CHARACTER).as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query("INSERT INTO game_durability_session_use_memberships VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL)").bind(id(SESSION).as_slice()).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants(control_role,world_id,house_key,operation) SELECT session_user,encode($1,'hex')::uuid,$2,operation FROM unnest(ARRAY[1,2,3]::smallint[]) operation").bind(id(WORLD).as_slice()).bind(HOUSE).execute(&h.pool).await?;
    let assignment = h
        .root
        .assign_house_scope(HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes([8; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            actor_role: "oteryn_test_admin".into(),
            command: HouseScopeAssignmentCommand::Assign {
                house: house()?,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    assert!(matches!(
        assignment,
        HouseScopeAssignmentOutcome::Committed(_)
    ));
    let seal = h.recovery.seal_current().map_err(debug)?;
    let auth = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let r = entry_request(60)?;
    assert!(matches!(
        h.root
            .prepare_house_entry(&auth, &h.node, r)
            .await
            .map_err(debug)?,
        HousePrepareOutcome::Prepared(_)
    ));
    Ok(())
}
#[test]
fn real_prepared_entrant_blocks_custody_and_fence_prevents_late_commit() -> TestResult {
    run("hm_entrant", async |h| {
        setup(h, 1).await?;
        past_notice(h).await?;
        prepare_entry(h).await?;
        apply(h, freeze()?).await.map_err(debug)?;
        assert!(matches!(apply(h, step(0)?).await, Err(Error::Occupied)));
        assert_eq!(counts(h).await?, (1, 1, 0, 0));
        // A valid native COMMIT attempt must fail closed; the production access stub stays intact.
        let seal = h.recovery.seal_current().map_err(debug)?;
        let auth = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // The native COMMIT refusal may itself delete PREPARED. Retain its actual row
        // before COMMIT solely for the later negative guarded-INSERT attribution.
        let native_prepared:String=sqlx::query_scalar("SELECT to_jsonb(h)::text FROM game_house_scope_handoffs h WHERE handoff_id=encode($1,'hex')::uuid").bind(id(60).as_slice()).fetch_one(&h.pool).await?;
        let outcome = h
            .root
            .commit_house_entry(
                &auth,
                &h.node,
                HouseEntryCommit {
                    handoff_id: id(60),
                    destination_game_session_id: GameSessionId::decode(&id(61)).map_err(debug)?,
                    committed_at: 200,
                },
            )
            .await;
        assert!(!matches!(outcome, Ok(HouseCommitOutcome::Committed(_))));
        // Positive control above reached native PREPARED. Abort any retained row through its real owner,
        // then a fresh occurrence reaches the INSERT and is rejected by our DB fence.
        h.root
            .abort_house_entry(&auth, &h.node, id(60))
            .await
            .map_err(debug)?;
        let late = h
            .root
            .prepare_house_entry(&auth, &h.node, entry_request(62)?)
            .await;
        // Native DurabilityError intentionally retains only SQLSTATE class, not message.
        assert!(
            matches!(
                late,
                Err(
                    crate::durability::house_scope_handoff::HouseHandoffError::Unavailable(
                        crate::durability::DurabilityError::Database(_)
                    )
                )
            ),
            "{late:?}"
        );
        // Attribute the boundary separately using the actual guarded native INSERT. Copy
        // the previous valid source row, varying only handoff occurrence/state and relying
        // on the very same active trigger; no fake access authority or disabled guard.
        let guarded=sqlx::query("INSERT INTO game_house_scope_handoffs SELECT (jsonb_populate_record(NULL::game_house_scope_handoffs,$1::text::jsonb||jsonb_build_object('handoff_id',encode($2,'hex')::uuid))).*").bind(native_prepared).bind(id(63).as_slice()).execute(&h.pool).await.expect_err("frozen native entry must refuse");
        let sqlx::Error::Database(guarded) = guarded else {
            return Err("expected database guard refusal".into());
        };
        assert_eq!(guarded.code().as_deref(), Some("23514"));
        assert!(
            guarded
                .message()
                .contains("house disposition content fence")
        );

        Ok(())
    })
}
#[test]
fn overfull_native_house_is_refused_before_full_census_capture() -> TestResult {
    run("hm_bound", async |h| {
        setup(h, 2_001).await?;
        past_notice(h).await?;
        assert!(matches!(
            apply(h, freeze()?).await,
            Err(Error::InvalidCustody)
        ));
        assert_eq!(h.count("game_house_disposition_items").await?, 0);
        assert_eq!(h.count("game_item_house_interior_locations").await?, 2_001);
        Ok(())
    })
}

#[test]
fn actual_committed_house_session_blocks_after_reservation_is_released() -> TestResult {
    run("hm_canonical", async |h| {
        setup(h, 1).await?;
        past_notice(h).await?;
        prepare_entry(h).await?;
        // Test-only replacement of the native closed access stub: derive OWNER from actual
        // 0038 ownership and closure from the private fence, never a supplied role/open flag.
        // This qualifies the native admission/census consumer, not HOUSE-1a/ACL activation.
        sqlx::raw_sql("CREATE OR REPLACE FUNCTION game_house_access(p_world_id UUID,p_house_key TEXT,p_character_id UUID) RETURNS TABLE(role TEXT,content_fenced BOOLEAN,acl_revision NUMERIC(20,0),guild_revisions JSONB) LANGUAGE sql AS $$ SELECT 'OWNER'::text,EXISTS(SELECT 1 FROM game_house_dispositions d WHERE d.world_id=p_world_id AND d.house_key=p_house_key AND d.state IN(2,3)),0::numeric,NULL::jsonb FROM game_house_ownership o WHERE o.world_id=p_world_id AND o.house_key=p_house_key AND o.owner_character_id=p_character_id FOR SHARE OF o $$").execute(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let auth = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .commit_house_entry(
                    &auth,
                    &h.node,
                    HouseEntryCommit {
                        handoff_id: id(60),
                        destination_game_session_id: GameSessionId::decode(&id(61))
                            .map_err(debug)?,
                        committed_at: 200
                    }
                )
                .await
                .map_err(debug)?,
            HouseCommitOutcome::Committed(_)
        ));
        apply(h, freeze()?).await.map_err(debug)?;
        assert!(matches!(apply(h, step(0)?).await, Err(Error::Occupied)));
        // Boundary-specific negative operator: trusted DB-owner fixture releases just the
        // retained reservation, with native transition guards active. The actual canonical
        // house session produced above remains live. This is not an exit/placement claim.
        sqlx::query("UPDATE game_house_scope_handoffs SET state=3,released_at=201 WHERE handoff_id=encode($1,'hex')::uuid").bind(id(60).as_slice()).execute(&h.pool).await?;
        let reservations: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_house_scope_handoffs WHERE state IN(1,2)",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(reservations, 0);
        assert!(matches!(apply(h, step(0)?).await, Err(Error::Occupied)));
        assert_eq!(counts(h).await?, (1, 1, 0, 0));
        Ok(())
    })
}

#[test]
fn another_current_world_channel_can_process_an_unloaded_house() -> TestResult {
    run("hm_other_channel", async |h| {
        setup(h, 1).await?;
        past_notice(h).await?;
        let node = crate::bestiary_postgres_harness::register(&h.root, 2).await?;
        let channel = crate::foundation::ChannelId::decode(&id(44)).map_err(debug)?;
        let execution_scope = crate::foundation::RuntimeScopeRefV1::channel(
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            channel,
        );
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) SELECT session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,operation FROM unnest(ARRAY[1,2,3]::smallint[]) operation").bind(id(WORLD).as_slice()).bind(id(44).as_slice()).execute(&h.pool).await?;
        let assigned = h
            .writer
            .submit(
                &crate::durability::runtime_scope_assignment::AssignmentRequest {
                    operation_key: OperationKey::from_bytes([19; 32]),
                    actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                    command:
                        crate::durability::runtime_scope_assignment::AssignmentCommand::Assign {
                            scope: execution_scope,
                            target: node.fact(),
                        },
                },
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            assigned,
            crate::durability::runtime_scope_assignment::AssignmentOutcome::Committed(_)
        ));
        let p = Pass::Freeze {
            operation: id(OP),
            house: house()?,
            execution_scope,
            scope_generation: 1,
        };
        // Node1 is genuinely current but does not hold Channel44; the independent assignment
        // check must refuse it. No House assignment exists or is manufactured for either node.
        assert!(matches!(
            apply(h, p.clone()).await,
            Err(Error::AuthorityRejected)
        ));
        let seal = h.recovery.seal_current().map_err(debug)?;
        let auth = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .run_private_house_move_out(&auth, &node, p)
                .await
                .map_err(debug)?,
            Outcome::Frozen(_)
        ));
        let bad = Pass::Step {
            operation: id(OP),
            house: house()?,
            execution_scope: crate::foundation::RuntimeScopeRefV1::channel(
                WorldId::decode(&id(84)).map_err(debug)?,
                channel,
            ),
            scope_generation: 1,
            step: 0,
        };
        assert!(matches!(
            h.root.run_private_house_move_out(&auth, &node, bad).await,
            Err(Error::AuthorityRejected)
        ));
        let p = Pass::Step {
            operation: id(OP),
            house: house()?,
            execution_scope,
            scope_generation: 1,
            step: 0,
        };
        assert!(matches!(
            h.root
                .run_private_house_move_out(&auth, &node, p)
                .await
                .map_err(debug)?,
            Outcome::Step {
                view: crate::durability::house_move_out::DispositionView { drained: true, .. },
                ..
            }
        ));
        Ok(())
    })
}

#[test]
fn owner_cancel_is_fenced_retained_and_allows_a_fresh_notice() -> TestResult {
    run("hm_cancel", async |h| {
        setup(h, 1).await?;
        apply(h, notice(30)?).await.map_err(debug)?;
        let Pass::Cancel {
            mut fence,
            command: cancel_command,
            request,
        } = cancellation(101)?
        else {
            return Err("cancel".into());
        };
        fence.character_lease_generation = 2;
        assert!(matches!(
            apply(
                h,
                Pass::Cancel {
                    fence,
                    command: cancel_command,
                    request
                }
            )
            .await,
            Err(Error::AuthorityRejected)
        ));
        assert!(matches!(
            apply(h, cancellation(100)?).await,
            Err(Error::ConflictingOperation)
        ));
        let first = apply(h, cancellation(101)?).await.map_err(debug)?;
        assert!(matches!(&first,Outcome::Cancelled(v) if v.cancelled&&!v.drained));
        assert_eq!(apply(h, cancellation(101)?).await.map_err(debug)?, first);
        assert!(matches!(apply(h, freeze()?).await, Err(Error::Cancelled)));
        let Pass::Notice {
            fence, mut request, ..
        } = notice(1)?
        else {
            return Err("notice".into());
        };
        request.operation = id(71);
        assert!(matches!(
            apply(
                h,
                Pass::Notice {
                    fence,
                    command: command(101)?,
                    request: request.clone()
                }
            )
            .await,
            Err(Error::ConflictingOperation)
        ));
        let fresh = apply(
            h,
            Pass::Notice {
                fence,
                command: command(102)?,
                request,
            },
        )
        .await
        .map_err(debug)?;
        assert!(matches!(fresh,Outcome::Notice(v) if !v.cancelled));
        assert_eq!(apply(h, cancellation(101)?).await.map_err(debug)?, first);
        let Pass::Cancel {
            fence,
            command,
            mut request,
        } = cancellation(101)?
        else {
            return Err("cancel".into());
        };
        request.operation = id(71);
        assert!(matches!(
            apply(
                h,
                Pass::Cancel {
                    fence,
                    command,
                    request
                }
            )
            .await,
            Err(Error::ConflictingOperation)
        ));
        let rows: Vec<(String, i16)> =
            sqlx::query_as("SELECT cause,state FROM game_house_dispositions ORDER BY operation_id")
                .fetch_all(&h.pool)
                .await?;
        assert_eq!(rows, vec![("MOVE_OUT".into(), 4), ("MOVE_OUT".into(), 1)]);
        assert_eq!(counts(h).await?, (1, 1, 0, 0));
        assert!(
            sqlx::query(
                "UPDATE game_house_dispositions SET cancelled_at_ms=cancelled_at_ms+1 WHERE state=4"
            )
            .execute(&h.pool)
            .await
            .is_err()
        );
        Ok(())
    })
}
#[test]
fn overdue_cancel_races_freeze_without_reopening_or_returning_items() -> TestResult {
    run("hm_cancel_due", async |h| {
        setup(h, 1).await?;
        past_notice(h).await?;
        let (cancel, frozen) = race_passes(h, cancellation(101)?, freeze()?).await?;
        assert!(
            matches!(cancel, Err(Error::CancellationClosed)),
            "{cancel:?}"
        );
        assert!(matches!(frozen, Ok(Outcome::Frozen(_))), "{frozen:?}");
        assert!(matches!(
            apply(h, cancellation(102)?).await,
            Err(Error::CancellationClosed)
        ));
        assert_eq!(counts(h).await?, (1, 1, 0, 0));
        Ok(())
    })
}
#[test]
fn before_due_cancel_races_not_due_freeze_as_one_retained_successor() -> TestResult {
    run("hm_cancel_race", async |h| {
        setup(h, 1).await?;
        apply(h, notice(1)?).await.map_err(debug)?;
        let (cancel, frozen) = race_passes(h, cancellation(101)?, freeze()?).await?;
        assert!(matches!(cancel, Ok(Outcome::Cancelled(_))), "{cancel:?}");
        assert!(
            matches!(frozen, Err(Error::NotDue | Error::Cancelled)),
            "{frozen:?}"
        );
        assert_eq!(counts(h).await?, (1, 1, 0, 0));
        Ok(())
    })
}

#[test]
fn current_successor_connection_can_cancel_an_older_notice() -> TestResult {
    run("hm_cancel_successor", async |h| {
        setup(h, 0).await?;
        apply(h, notice(1)?).await.map_err(debug)?;
        // Boundary-specific persisted successor operator with native guards active. This
        // qualifies current-generation consumption, not the separate reconnect workflow.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=2 WHERE game_session_id=encode($1,'hex')::uuid").bind(id(SESSION).as_slice()).execute(&h.pool).await?;
        assert!(matches!(
            apply(h, cancellation(101)?).await,
            Err(Error::AuthorityRejected)
        ));
        let Pass::Cancel {
            mut fence,
            command,
            request,
        } = cancellation(101)?
        else {
            return Err("cancel".into());
        };
        fence.connection_generation =
            crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
        let current = Pass::Cancel {
            fence,
            command,
            request,
        };
        let first = apply(h, current.clone()).await.map_err(debug)?;
        assert!(matches!(first, Outcome::Cancelled(_)));
        assert_eq!(apply(h, current.clone()).await.map_err(debug)?, first);
        let Pass::Cancel { fence, .. } = current else {
            return Err("cancel".into());
        };
        let Pass::Notice { mut request, .. } = notice(1)? else {
            return Err("notice".into());
        };
        request.operation = id(71);
        apply(
            h,
            Pass::Notice {
                fence,
                command: self::command(102)?,
                request,
            },
        )
        .await
        .map_err(debug)?;
        // One current-owner revision operator on a still-pending notice, with all native
        // house successor guards active; session/generation/due and actor remain valid.
        sqlx::query("UPDATE game_house_ownership SET ownership_revision=2,acquisition_receipt_ref=encode($1,'hex')::uuid").bind(id(91).as_slice()).execute(&h.pool).await?;
        assert!(matches!(
            apply(
                h,
                Pass::Cancel {
                    fence,
                    command: self::command(103)?,
                    request: MoveOutCancellation {
                        operation: id(71),
                        house: house()?,
                        ownership_revision: 1
                    }
                }
            )
            .await,
            Err(Error::OwnershipChanged)
        ));
        Ok(())
    })
}
