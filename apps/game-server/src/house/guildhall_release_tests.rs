//! Native qualification leaf. Mount as a child of the genuine D2136
//! `guild::disband_terminal_tests` in the private native snapshot, after mounting the
//! exact D2136/D2139/D2153, C2137/C2141 and BED2147 production sources.
//! `super::run/install/job` below are those real D fixtures, not replacement contexts.
//! This file is deliberately unregistered until that exact-source composition exists.
use super::*;
use crate::durability::house_ownership::{
    GUILD_DISBAND_RELEASE_SCHEMA_SQL, GuildHallReleaseError, GuildHallReleaseOutcome,
    ORDINARY_PROPERTY_SCHEMA_SQL, advance_guild_disband_release_in_transaction,
    assert_guild_disband_released_in_transaction, reserve_guild_disband_recipients_in_transaction,
};
use sqlx::{Acquire, Postgres, Row, Transaction};

const HOUSE: &str = "oteryn:content.house.thais_1";
fn item(n: usize) -> [u8; 16] {
    let mut i = id(100);
    i[14] = (n >> 8) as u8;
    i[15] = n as u8;
    i
}
fn placement(n: usize) -> [u8; 16] {
    let mut i = id(220);
    i[14] = (n >> 8) as u8;
    i[15] = n as u8;
    i
}

async fn setup(h: &Harness, count: usize) -> TestResult {
    for schema in [
        ORDINARY_PROPERTY_SCHEMA_SQL,
        crate::durability::bed_sleeper::PRIVATE_SLEEPER_SCHEMA_SQL,
        crate::durability::house_move_out::PRIVATE_DISPOSITION_SCHEMA,
        GUILD_DISBAND_RELEASE_SCHEMA_SQL,
    ] {
        sqlx::raw_sql(schema).execute(&h.pool).await?;
    }
    // Initial retained state only. Acquisition/Premium/economy are not this release case.
    // No guard is disabled during reserve, real D context construction, release or commit.
    let mut tx = h.pool.begin().await?;
    tx.execute("SET LOCAL session_replication_role=replica")
        .await?;
    sqlx::query("INSERT INTO game_house_properties(world_id,house_key,state,owner_kind,owner_guild_id,paid_until,revision,acl_revision,ownership_revision,acquisition_receipt_ref) VALUES(encode($1,'hex')::uuid,$2,'OWNED','GUILD',encode($3,'hex')::uuid,9999999999999,3,0,1,encode($4,'hex')::uuid)")
        .bind(id(WORLD).as_slice()).bind(HOUSE).bind(id(90).as_slice()).bind(id(91).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,owner_guild_id,ownership_revision,acquisition_receipt_ref) VALUES(encode($1,'hex')::uuid,$2,NULL,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
        .bind(id(WORLD).as_slice()).bind(HOUSE).bind(id(90).as_slice()).bind(id(91).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_bed_revisions(world_id,house_key,revision) VALUES(encode($1,'hex')::uuid,$2,1)")
        .bind(id(WORLD).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_bed_sleepers(world_id,house_key,bed_x,bed_y,bed_z,character_id,lease_generation,occupied_variant,slept_at) VALUES(encode($1,'hex')::uuid,$2,1,2,0,encode($3,'hex')::uuid,1,'male',1)")
        .bind(id(WORLD).as_slice()).bind(HOUSE).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_acl_receipts(receipt_id,world_id,house_key,list_id,character_id,game_session_id,command_id,ownership_revision,revision_before,revision_after,allow_everyone,members) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,-1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,99,1,0,1,true,'{}')")
        .bind(id(95).as_slice()).bind(id(WORLD).as_slice()).bind(HOUSE).bind(id(CHARACTER).as_slice()).bind(id(50).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_acl(world_id,house_key,list_id,revision,allow_everyone,members,last_receipt) VALUES(encode($1,'hex')::uuid,$2,-1,1,true,'{}',encode($3,'hex')::uuid)")
        .bind(id(WORLD).as_slice()).bind(HOUSE).bind(id(95).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_house_instances(world_id,house_key,house_revision,instance_id,compatible_content_digest,qualified_placement_digest,binding_receipt_ref) VALUES(encode($1,'hex')::uuid,$2,'r1',encode($3,'hex')::uuid,$4,$4,encode($5,'hex')::uuid)")
        .bind(id(WORLD).as_slice()).bind(HOUSE).bind(id(96).as_slice()).bind([1_u8;32].as_slice()).bind(id(97).as_slice()).execute(&mut *tx).await?;
    for n in 0..count {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:content.item.vase','r1',1,1,encode($3,'hex')::uuid)")
            .bind(item(n).as_slice()).bind(id(WORLD).as_slice()).bind(placement(n).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_interior_locations(item_instance_id,world_id,house_key,spatial_position,stack_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,encode($6,'hex')::uuid)")
            .bind(item(n).as_slice()).bind(id(WORLD).as_slice()).bind(HOUSE).bind([1_u8,2,3].as_slice()).bind((n+1) as i64).bind(placement(n).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
            .bind(item(n).as_slice()).bind(id(WORLD).as_slice()).bind(HOUSE).bind(id(CHARACTER).as_slice()).bind(placement(n).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

// All real D occurrence locks are held before recipient pins. The production D caller
// must apply the separately proposed occurrence/spine split; this fixture does not claim
// the old unsplit D orchestrator has already activated the C participant.
async fn occurrences(
    tx: &mut Transaction<'_, Postgres>,
    h: &Harness,
    root: &crate::durability::DurabilityRoot,
) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = root.open_character_authority(&seal).await.map_err(debug)?;
    let record = authority.record_for(root).map_err(debug)?;
    crate::durability::character_authority::assert_recovery_fence(tx, &record)
        .await
        .map_err(debug)?;
    crate::durability::db::lock_admission_relations(tx)
        .await
        .map_err(debug)?;
    let request = job()?;
    assert_guild_job_world(tx, &h.node, request.world_id)
        .await
        .map_err(debug)?;
    crate::durability::guildhall::lock_guild_disband_release_in_transaction(
        tx,
        &request.hall_identity,
    )
    .await
    .map_err(debug)?;
    crate::durability::guild_bank::lock_guild_disband_payout_occurrence(tx, &request.bank_identity)
        .await
        .map_err(debug)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('guild-disband-terminal:'||encode($1,'hex'),0))")
        .bind(id(90).as_slice()).execute(&mut **tx).await?;
    Ok(())
}
async fn pass(h: &Harness, step: u64, reservation: u8, commit: bool) -> TestResult<(u64, bool)> {
    pass_as(h, step, reservation, commit, false).await
}
async fn pass_as(
    h: &Harness,
    step: u64,
    reservation: u8,
    commit: bool,
    runtime_role: bool,
) -> TestResult<(u64, bool)> {
    pass_root(h, &h.root, step, reservation, commit, runtime_role).await
}
async fn pass_root(
    h: &Harness,
    root: &crate::durability::DurabilityRoot,
    step: u64,
    reservation: u8,
    commit: bool,
    runtime_role: bool,
) -> TestResult<(u64, bool)> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = root.open_character_authority(&seal).await.map_err(debug)?;
    let request = job()?;
    let mut tx = h.pool.begin().await?;
    occurrences(&mut tx, h, root).await?;
    if runtime_role {
        tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
    }
    let reserved = reserve_guild_disband_recipients_in_transaction(
        &mut tx,
        request.world_id,
        request.guild_id,
        id(92),
        step,
        id(reservation),
    )
    .await
    .map_err(debug)?;
    let context = lock_guild_disband_transaction(&mut tx, root, &authority, &h.node, &request)
        .await
        .map_err(debug)?
        .ok_or("real disband context missing")?;
    let result = advance_guild_disband_release_in_transaction(&mut tx, &context, reserved)
        .await
        .map_err(debug)?;
    let result = match result {
        GuildHallReleaseOutcome::Progress { moved, next_step } => {
            assert_eq!(next_step, step + 1);
            (moved, false)
        }
        GuildHallReleaseOutcome::Released(proof) => {
            assert_guild_disband_released_in_transaction(&mut tx, &context, &proof)
                .await
                .map_err(debug)?;
            tx.execute("RESET ROLE").await?;
            let released:i16=sqlx::query_scalar("SELECT state FROM game_house_guild_dispositions WHERE operation_id=encode($1,'hex')::uuid").bind(id(92).as_slice()).fetch_one(&mut *tx).await?;
            assert_eq!(released, 5);
            // This is the actual common native entry gate, not a replacement predicate.
            sqlx::query(
                "SELECT game_house_disposition_fence_check(encode($1,'hex')::uuid,$2,NULL,false)",
            )
            .bind(id(WORLD).as_slice())
            .bind(HOUSE)
            .execute(&mut *tx)
            .await?;
            (0, true)
        }
    };
    if commit {
        tx.commit().await?;
    } else {
        tx.rollback().await?;
    }
    Ok(result)
}

#[test]
fn guild_house_real_inbox_release_rollback_then_commit_reload() -> TestResult {
    run("guild_house_atomic", async |h| {
        setup(h, 2).await?;
        assert!(pass(h, 0, 93, false).await?.1);
        assert_eq!(h.count("game_item_house_interior_locations").await?, 2);
        assert_eq!(h.count("game_character_inbox_deliveries").await?, 0);
        assert_eq!(h.count("game_house_terminal_releases").await?, 0);
        assert!(pass(h, 0, 93, true).await?.1);
        assert_eq!(h.count("game_item_house_interior_locations").await?, 0);
        assert_eq!(h.count("game_item_house_reclaim_provenance").await?, 0);
        assert_eq!(h.count("game_character_inbox_deliveries").await?, 2);
        assert_eq!(h.count("game_item_character_inbox_locations").await?, 2);
        assert_eq!(h.count("game_house_acl").await?, 0);
        assert_eq!(h.count("game_house_acl_receipts").await?, 1);
        assert_eq!(h.count("game_world_house_instances").await?, 1);
        let cause:String=sqlx::query_scalar("SELECT freed_cause FROM game_house_bed_sleepers WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(cause, "disposition");
        let p=sqlx::query("SELECT state,owner_guild_id IS NULL AS empty,ownership_revision::text,acquisition_receipt_ref=encode($1,'hex')::uuid AS receipt FROM game_house_properties WHERE house_key=$2")
            .bind(id(91).as_slice()).bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(p.try_get::<String, _>("state")?, "VACANT");
        assert!(p.try_get::<bool, _>("empty")?);
        assert!(p.try_get::<bool, _>("receipt")?);
        assert_eq!(p.try_get::<String, _>("ownership_revision")?, "2");
        Ok(())
    })
}

#[test]
fn guild_house_bounded_101_retains_actual_fence_until_final_real_delivery() -> TestResult {
    run("guild_house_101", async |h| {
        setup(h, 101).await?;
        assert_eq!(pass(h, 0, 93, true).await?, (100, false));
        assert_eq!(h.count("game_item_house_interior_locations").await?, 1);
        assert_eq!(h.count("game_character_inbox_deliveries").await?, 100);
        for statement in [
            "SELECT game_house_disposition_fence_check(encode($1,'hex')::uuid,$2,NULL,false)",
            "DELETE FROM game_item_house_reclaim_provenance WHERE world_id=encode($1,'hex')::uuid AND house_key=$2",
            "UPDATE game_item_house_interior_locations SET spatial_position=spatial_position WHERE world_id=encode($1,'hex')::uuid AND house_key=$2",
            "UPDATE game_item_instances SET quantity=quantity WHERE item_instance_id IN(SELECT item_instance_id FROM game_item_house_interior_locations WHERE world_id=encode($1,'hex')::uuid AND house_key=$2)",
        ] {
            let mut tx = h.pool.begin().await?;
            let error = sqlx::query(statement)
                .bind(id(WORLD).as_slice())
                .bind(HOUSE)
                .execute(&mut *tx)
                .await
                .err()
                .ok_or("native frozen mutation passed")?;
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("23514")
            );
            tx.rollback().await?;
        }
        let restarted = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        assert!(pass_root(h, &restarted, 1, 94, true, false).await?.1);
        assert_eq!(h.count("game_character_inbox_deliveries").await?, 101);
        assert_eq!(h.count("game_house_terminal_releases").await?, 1);
        Ok(())
    })
}

#[test]
fn guild_house_reservation_and_release_proof_reject_savepoint_rollback() -> TestResult {
    run("guild_house_savepoints", async |h| {
        setup(h, 1).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = job()?;
        let mut tx = h.pool.begin().await?;
        occurrences(&mut tx, h, &h.root).await?;
        let reserved = {
            let mut sp = tx.begin().await?;
            let r = reserve_guild_disband_recipients_in_transaction(
                &mut sp,
                request.world_id,
                request.guild_id,
                id(92),
                0,
                id(93),
            )
            .await
            .map_err(debug)?;
            sp.rollback().await?;
            r
        };
        let context =
            lock_guild_disband_transaction(&mut tx, &h.root, &authority, &h.node, &request)
                .await
                .map_err(debug)?
                .ok_or("context")?;
        assert!(matches!(
            advance_guild_disband_release_in_transaction(&mut tx, &context, reserved).await,
            Err(GuildHallReleaseError::NotCurrent)
        ));
        tx.rollback().await?;
        let mut tx = h.pool.begin().await?;
        occurrences(&mut tx, h, &h.root).await?;
        let reserved = reserve_guild_disband_recipients_in_transaction(
            &mut tx,
            request.world_id,
            request.guild_id,
            id(92),
            0,
            id(93),
        )
        .await
        .map_err(debug)?;
        let context =
            lock_guild_disband_transaction(&mut tx, &h.root, &authority, &h.node, &request)
                .await
                .map_err(debug)?
                .ok_or("context")?;
        let proof = {
            let mut sp = tx.begin().await?;
            let GuildHallReleaseOutcome::Released(p) =
                advance_guild_disband_release_in_transaction(&mut sp, &context, reserved)
                    .await
                    .map_err(debug)?
            else {
                return Err("not released".into());
            };
            sp.rollback().await?;
            p
        };
        assert!(matches!(
            assert_guild_disband_released_in_transaction(&mut tx, &context, &proof).await,
            Err(GuildHallReleaseError::NotCurrent)
        ));
        tx.rollback().await?;
        assert_eq!(h.count("game_item_house_interior_locations").await?, 1);
        Ok(())
    })
}

async fn footprint(tx: &mut Transaction<'_, Postgres>) -> TestResult<Vec<String>> {
    let mut values = Vec::new();
    for table in [
        "game_house_properties",
        "game_house_ownership",
        "game_house_acl",
        "game_house_acl_receipts",
        "game_world_house_instances",
        "game_house_bed_revisions",
        "game_house_bed_sleepers",
        "game_item_instances",
        "game_item_house_interior_locations",
        "game_item_house_reclaim_provenance",
        "game_character_inbox_counters",
        "game_character_inbox_deliveries",
        "game_item_character_inbox_locations",
        "game_house_guild_dispositions",
        "game_house_guild_disposition_items",
        "game_house_guild_disposition_steps",
        "game_house_guild_recipient_reservations",
        "game_house_terminal_releases",
        "game_house_guild_authority_stamps",
    ] {
        values.push(sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT coalesce(jsonb_agg(j ORDER BY j::text),'[]'::jsonb)::text FROM (SELECT to_jsonb(t) AS j FROM {table} t) snapshot"))).fetch_one(&mut **tx).await?);
    }
    Ok(values)
}

#[test]
fn guild_house_runtime_exact_ports_without_direct_table_writes() -> TestResult {
    run("guild_house_roles", async |h| {
        setup(h, 1).await?;
        for table in [
            "game_house_properties",
            "game_house_guild_dispositions",
            "game_house_guild_recipient_reservations",
            "game_house_terminal_releases",
            "game_house_guild_authority_stamps",
            "game_house_acl",
            "game_house_bed_sleepers",
        ] {
            let mut tx = h.pool.begin().await?;
            tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
            let error = sqlx::query(sqlx::AssertSqlSafe(format!(
                "DELETE FROM {table} WHERE false"
            )))
            .execute(&mut *tx)
            .await
            .err()
            .ok_or("runtime acquired direct House DML")?;
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("42501")
            );
            tx.rollback().await?;
        }
        assert!(pass_as(h, 0, 93, true, true).await?.1);
        Ok(())
    })
}

#[test]
fn guild_house_native_delivery_error_restores_participant_before_caller_commit() -> TestResult {
    run("guild_house_fault", async |h| {
        setup(h, 2).await?;
        // Deterministic fault after the first genuine Inbox effect. This trigger adds a
        // failure; it neither disables a native guard nor supplies a fake delivery proof.
        sqlx::raw_sql("CREATE FUNCTION house_test_second_delivery_error() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF (SELECT count(*) FROM game_character_inbox_deliveries)>=2 THEN RAISE EXCEPTION 'injected second delivery failure' USING ERRCODE='OHG99';END IF;RETURN NEW;END $$; CREATE TRIGGER house_test_second_delivery_error AFTER INSERT ON game_character_inbox_deliveries FOR EACH ROW EXECUTE FUNCTION house_test_second_delivery_error();").execute(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = job()?;
        let mut tx = h.pool.begin().await?;
        occurrences(&mut tx, h, &h.root).await?;
        let reserved = reserve_guild_disband_recipients_in_transaction(
            &mut tx,
            request.world_id,
            request.guild_id,
            id(92),
            0,
            id(93),
        )
        .await
        .map_err(debug)?;
        let context =
            lock_guild_disband_transaction(&mut tx, &h.root, &authority, &h.node, &request)
                .await
                .map_err(debug)?
                .ok_or("context")?;
        let before = footprint(&mut tx).await?;
        let error = advance_guild_disband_release_in_transaction(&mut tx, &context, reserved)
            .await
            .err()
            .ok_or("injected native delivery did not fail")?;
        let GuildHallReleaseError::Database(error) = error else {
            return Err("wrong fault outcome".into());
        };
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("OHG99")
        );
        tx.execute("RESET ROLE").await?;
        assert_eq!(footprint(&mut tx).await?, before);
        // Adversarial caller commits after Err: protected C SAVEPOINT still prevents a
        // partial Inbox/counter/census/BED/ACL/property/anchor effect from escaping.
        tx.commit().await?;
        assert_eq!(h.count("game_item_house_interior_locations").await?, 2);
        assert_eq!(h.count("game_character_inbox_deliveries").await?, 0);
        assert_eq!(h.count("game_house_terminal_releases").await?, 0);
        Ok(())
    })
}

#[test]
fn guild_house_missing_actual_native_hook_refuses_before_freeze() -> TestResult {
    run("guild_house_capability", async |h| {
        setup(h, 1).await?;
        sqlx::raw_sql(
            "ALTER TABLE game_item_instances DISABLE TRIGGER a_house_disposition_item_fence",
        )
        .execute(&h.pool)
        .await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = job()?;
        let mut tx = h.pool.begin().await?;
        occurrences(&mut tx, h, &h.root).await?;
        let reserved = reserve_guild_disband_recipients_in_transaction(
            &mut tx,
            request.world_id,
            request.guild_id,
            id(92),
            0,
            id(93),
        )
        .await
        .map_err(debug)?;
        let context =
            lock_guild_disband_transaction(&mut tx, &h.root, &authority, &h.node, &request)
                .await
                .map_err(debug)?
                .ok_or("context")?;
        let before = footprint(&mut tx).await?;
        assert!(matches!(
            advance_guild_disband_release_in_transaction(&mut tx, &context, reserved).await,
            Err(GuildHallReleaseError::MissingDependency(_))
        ));
        assert_eq!(footprint(&mut tx).await?, before);
        tx.commit().await?;
        assert_eq!(h.count("game_house_guild_dispositions").await?, 0);
        Ok(())
    })
}

#[test]
fn guild_house_real_d_context_after_rollback_reacquires_guild_lock() -> TestResult {
    run("guild_house_d_lock", async |h| {
        setup(h, 1).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = job()?;
        let mut tx = h.pool.begin().await?;
        occurrences(&mut tx, h, &h.root).await?;
        let reserved = reserve_guild_disband_recipients_in_transaction(
            &mut tx,
            request.world_id,
            request.guild_id,
            id(92),
            0,
            id(93),
        )
        .await
        .map_err(debug)?;
        let context = {
            let mut sp = tx.begin().await?;
            let c = lock_guild_disband_transaction(&mut sp, &h.root, &authority, &h.node, &request)
                .await
                .map_err(debug)?
                .ok_or("context")?;
            sp.rollback().await?;
            c
        };
        // The genuine context's xid/current revision still passes despite lost Guild locks.
        context.assert_current(&mut tx).await.map_err(debug)?;
        let mut other = h.pool.begin().await?;
        sqlx::query(
            "SELECT guild_id FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE",
        )
        .bind(id(90).as_slice())
        .fetch_one(&mut *other)
        .await?;
        tx.execute("SET LOCAL lock_timeout='100ms'").await?;
        let before = footprint(&mut tx).await?;
        let error = advance_guild_disband_release_in_transaction(&mut tx, &context, reserved)
            .await
            .err()
            .ok_or("C trusted rolled-back D locks")?;
        let GuildHallReleaseError::Database(error) = error else {
            return Err("expected actual lock reacquisition failure".into());
        };
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("55P03")
        );
        assert_eq!(footprint(&mut tx).await?, before);
        other.rollback().await?;
        tx.commit().await?;
        Ok(())
    })
}

#[test]
fn guild_house_real_context_rechecks_recovery_after_reservation() -> TestResult {
    run("guild_house_recovery", async |h| {
        setup(h, 1).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = job()?;
        let mut tx = h.pool.begin().await?;
        occurrences(&mut tx, h, &h.root).await?;
        let reserved = reserve_guild_disband_recipients_in_transaction(
            &mut tx,
            request.world_id,
            request.guild_id,
            id(92),
            0,
            id(93),
        )
        .await
        .map_err(debug)?;
        let context =
            lock_guild_disband_transaction(&mut tx, &h.root, &authority, &h.node, &request)
                .await
                .map_err(debug)?
                .ok_or("context")?;
        // Same genuine admission replacement used by D2136's native negative case.
        // No C proof/context is manufactured, no existing admission row is rewritten.
        h.pool.execute("INSERT INTO game_character_recovery_admissions(authority_scope_id,recovery_generation,recovery_event_id,predecessor_generation,predecessor_digest,issued_at,issuer_identity,reconciled_at) SELECT authority_scope_id,recovery_generation+1,'10000000-0000-7000-8000-000000000001',recovery_generation,decode(repeat('00',32),'hex'),issued_at+1,issuer_identity,reconciled_at FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1").await?;
        let before = footprint(&mut tx).await?;
        assert!(matches!(
            advance_guild_disband_release_in_transaction(&mut tx, &context, reserved).await,
            Err(GuildHallReleaseError::GuildAuthority(_))
        ));
        assert_eq!(footprint(&mut tx).await?, before);
        tx.commit().await?;
        Ok(())
    })
}

#[test]
fn guild_house_real_context_rejects_foreign_world_and_canonical_resnapshot() -> TestResult {
    run("guild_house_world", async |h| {
        setup(h, 1).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = job()?;
        let mut tx = h.pool.begin().await?;
        occurrences(&mut tx, h, &h.root).await?;
        let other_world = crate::foundation::WorldId::decode(&id(99)).map_err(debug)?;
        let reserved = reserve_guild_disband_recipients_in_transaction(
            &mut tx,
            other_world,
            request.guild_id,
            id(92),
            0,
            id(93),
        )
        .await
        .map_err(debug)?;
        let context =
            lock_guild_disband_transaction(&mut tx, &h.root, &authority, &h.node, &request)
                .await
                .map_err(debug)?
                .ok_or("context")?;
        let before = footprint(&mut tx).await?;
        assert!(matches!(
            advance_guild_disband_release_in_transaction(&mut tx, &context, reserved).await,
            Err(GuildHallReleaseError::NotCurrent)
        ));
        assert_eq!(footprint(&mut tx).await?, before);
        tx.rollback().await?;
        let mut tx = h.pool.begin().await?;
        occurrences(&mut tx, h, &h.root).await?;
        let reserved = reserve_guild_disband_recipients_in_transaction(
            &mut tx,
            request.world_id,
            request.guild_id,
            id(92),
            0,
            id(93),
        )
        .await
        .map_err(debug)?;
        let context =
            lock_guild_disband_transaction(&mut tx, &h.root, &authority, &h.node, &request)
                .await
                .map_err(debug)?
                .ok_or("context")?;
        sqlx::query("UPDATE game_house_properties SET paid_until=paid_until+1,revision=revision+1 WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
            .bind(id(WORLD).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
        let before = footprint(&mut tx).await?;
        assert!(matches!(
            advance_guild_disband_release_in_transaction(&mut tx, &context, reserved).await,
            Err(GuildHallReleaseError::ResnapshotRequired)
        ));
        assert_eq!(footprint(&mut tx).await?, before);
        tx.commit().await?;
        Ok(())
    })
}

#[test]
fn guild_house_direct_sql_cannot_replace_root_possession_with_public_current_facts() -> TestResult {
    run("guild_house_sql_authority", async |h| {
        setup(h, 1).await?;
        let request = job()?;
        let mut tx = h.pool.begin().await?;
        occurrences(&mut tx, h, &h.root).await?;
        let _reservation = reserve_guild_disband_recipients_in_transaction(
            &mut tx,
            request.world_id,
            request.guild_id,
            id(92),
            0,
            id(93),
        )
        .await
        .map_err(debug)?;
        let binding: Vec<u8> = sqlx::query_scalar(
            "SELECT game_house_guild_reservation_binding(encode($1,'hex')::uuid)",
        )
        .bind(id(93).as_slice())
        .fetch_one(&mut *tx)
        .await?;
        let xid: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut *tx)
            .await?;
        let before = footprint(&mut tx).await?;
        let mut sp = tx.begin().await?;
        sp.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        // Even exact public Guild/World/xid/census bindings do not establish possession.
        // No protected authority/spine stamp exists from the genuine Root-provided proof.
        let error=sqlx::query("SELECT * FROM game_house_guild_advance(encode($1,'hex')::uuid,encode($2,'hex')::uuid,0,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6)")
            .bind(id(93).as_slice()).bind(id(92).as_slice()).bind(id(90).as_slice()).bind(id(WORLD).as_slice()).bind(binding).bind(xid)
            .fetch_one(&mut *sp).await.err().ok_or("public matching facts became Root authority")?;
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("OHG02")
        );
        sp.rollback().await?;
        assert_eq!(footprint(&mut tx).await?, before);
        tx.commit().await?;
        Ok(())
    })
}

#[test]
fn guild_house_disposition_preserves_prior_freed_bed_cause_and_time() -> TestResult {
    run("guild_house_bed_final", async |h| {
        setup(h, 1).await?;
        sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=2,freed_cause='owner_wake' WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
        assert!(pass(h, 0, 93, true).await?.1);
        let row=sqlx::query("SELECT freed_at,freed_cause FROM game_house_bed_sleepers WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(row.try_get::<i64, _>("freed_at")?, 2);
        assert_eq!(row.try_get::<String, _>("freed_cause")?, "owner_wake");
        Ok(())
    })
}

#[test]
fn guild_house_missing_protected_abi_refuses_before_recipient_marker() -> TestResult {
    run("guild_house_no_abi", async |h| {
        setup(h, 1).await?;
        sqlx::raw_sql("ALTER FUNCTION game_house_disposition_guild_fence_abi() RENAME TO house_test_unavailable_abi").execute(&h.pool).await?;
        let request = job()?;
        let mut tx = h.pool.begin().await?;
        occurrences(&mut tx, h, &h.root).await?;
        assert!(matches!(
            reserve_guild_disband_recipients_in_transaction(
                &mut tx,
                request.world_id,
                request.guild_id,
                id(92),
                0,
                id(93)
            )
            .await,
            Err(GuildHallReleaseError::MissingDependency(_))
        ));
        tx.commit().await?;
        assert_eq!(h.count("game_house_guild_recipient_reservations").await?, 0);
        assert_eq!(h.count("game_house_guild_authority_stamps").await?, 0);
        assert_eq!(h.count("game_house_guild_dispositions").await?, 0);
        Ok(())
    })
}
