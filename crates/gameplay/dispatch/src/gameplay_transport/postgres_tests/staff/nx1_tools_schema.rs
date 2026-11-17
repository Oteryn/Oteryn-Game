//! NX-1 negative SQL and exact-session fences, using a genuine off-host signature.
use super::*;
use crate::durability::staff::StaffWorldEffectState;

fn clock() -> Option<(u64, u64)> {
    let now = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis(),
    )
    .ok()?;
    Some((now, now))
}
fn refused_sql(result: Result<sqlx::postgres::PgQueryResult, sqlx::Error>) -> TestResult {
    let error = result.err().ok_or("tampered staff evidence accepted")?;
    assert_eq!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23514"),
        "{error:?}"
    );
    Ok(())
}
#[test]
fn nx1_staff_tools_fenced_ack_and_signed_sql_bindings_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let began = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "NX-1 schema case panicked")??;
    eprintln!(
        "NX-1 staff schema PostgreSQL duration: {:?}",
        began.elapsed()
    );
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let signing = Signing::new()?;
        let h = Harness::create_with_admission_seed(admin, "nx1_staff_schema", true, false).await?;
        prepare_current_session(&h).await?;
        signing.bootstrap(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let fence = owner
            .current_quest_fence(deps.admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("current fence")?
            .item_fence();
        let now = clock().ok_or("clock")?.0;
        let inspect = signing.request(
            2,
            &signing.config.newest_activation_sha256,
            90,
            now,
            rules::Action::Inspect {
                character_id: hex(&id(41)),
            },
        );
        let inspect_bytes = canonical(&inspect)?;
        let inspected = h
            .root
            .submit_staff_tools(
                &authority,
                owner.holder,
                fence,
                signing.sign(&inspect_bytes, false)?,
                false,
                clock,
            )
            .await?
            .ok_or("self inspect binding")?;
        assert_eq!(wire::decode_result(&inspected)?.code, wire::StaffCode::Ok);
        assert!(
            h.root
                .staff_tools_actor_authorized(&authority, owner.holder, fence, "operator", 6)
                .await?
        );
        assert!(
            !h.root
                .staff_tools_actor_authorized(&authority, owner.holder, fence, "other", 4)
                .await?
        );
        assert!(
            !h.root
                .staff_tools_actor_authorized(&authority, owner.holder, fence, "operator", 7)
                .await?
        );
        let operation = hex(&id(91));
        let request = signing.request(
            3,
            &sha(&inspect_bytes),
            91,
            now,
            rules::Action::Ghost {
                character_id: hex(&id(41)),
                enabled: true,
            },
        );
        let signed = signing.sign(&canonical(&request)?, false)?;
        let result = h
            .root
            .submit_staff_tools(&authority, owner.holder, fence, signed, false, clock)
            .await?
            .ok_or("signed ghost result")?;
        assert_eq!(wire::decode_result(&result)?.code, wire::StaffCode::Ok);
        assert_eq!(
            h.root
                .staff_world_effect_state(&authority, owner.holder, fence, &operation)
                .await?,
            Some(StaffWorldEffectState::Pending)
        );
        assert_eq!(
            h.root
                .staff_world_effect_target(&authority, owner.holder, fence, &operation)
                .await?,
            Some(crate::durability::staff::StaffWorldEffectTarget { session: fence.game_session_id, connection_generation: fence.connection_generation.get() })
        );
        let mut stale = fence;
        stale.connection_generation =
            crate::foundation::ConnectionGeneration::new(99).map_err(debug)?;
        assert!(
            h.root
                .staff_live_status(&authority, owner.holder, stale)
                .await
                .is_err()
        );
        assert!(
            h.root
                .ack_staff_world_effect(&authority, owner.holder, stale, &operation, true)
                .await
                .is_err()
        );
        assert_eq!(
            h.root
                .staff_world_effect_state(&authority, owner.holder, fence, &operation)
                .await?,
            Some(StaffWorldEffectState::Pending)
        );
        refused_sql(sqlx::query("INSERT INTO game_staff_world_effects(environment,operation_id,target_character_id,issuer_character_id,target_game_session_id,target_connection_generation,target_scope_ownership_generation,action) SELECT environment,operation_id,target_character_id,$2::uuid,target_game_session_id,target_connection_generation,target_scope_ownership_generation,action FROM game_staff_world_effects WHERE operation_id=$1::uuid")
            .bind(&operation).bind(hex(&id(42))).execute(&h.pool).await)?;
        refused_sql(sqlx::query("INSERT INTO game_staff_character_groups(environment,character_id,group_id,activation_sha256,principal,operation_id) SELECT environment,$2::uuid,group_id,activation_sha256,principal,operation_id FROM game_staff_character_groups WHERE character_id=$1::uuid")
            .bind(hex(&id(41))).bind(hex(&id(42))).execute(&h.pool).await)?;
        refused_sql(sqlx::query("UPDATE game_staff_world_effects SET action=action||'{\"enabled\":false}'::jsonb WHERE operation_id=$1::uuid")
            .bind(&operation).execute(&h.pool).await)?;
        h.root
            .ack_staff_world_effect(&authority, owner.holder, fence, &operation, true)
            .await?;
        let status = h
            .root
            .staff_live_status(&authority, owner.holder, fence)
            .await?;
        assert!(status.ghost);
        assert!(!status.invisible);
        assert_eq!(status.group, 6);
        assert!(!status.banned);
        assert!(!status.kicked);
        refused_sql(
            sqlx::query(
                "UPDATE game_staff_actor_flags SET invisible=true WHERE operation_id=$1::uuid",
            )
            .bind(&operation)
            .execute(&h.pool)
            .await,
        )?;
        refused_sql(
            sqlx::query(
                "UPDATE game_staff_character_groups SET group_id=4 WHERE character_id=$1::uuid",
            )
            .bind(hex(&id(41)))
            .execute(&h.pool)
            .await,
        )?;
        refused_sql(
            sqlx::query(
                "UPDATE game_staff_world_effects SET success=false WHERE operation_id=$1::uuid",
            )
            .bind(&operation)
            .execute(&h.pool)
            .await,
        )?;
        h.root
            .ack_staff_world_effect(&authority, owner.holder, fence, &operation, false)
            .await?;
        assert_eq!(
            h.root
                .staff_world_effect_state(&authority, owner.holder, fence, &operation)
                .await?,
            Some(StaffWorldEffectState::Applied)
        );
        assert!(
            h.root
                .staff_live_status(&authority, owner.holder, fence)
                .await?
                .ghost
        );
        // An ordinary node sees the same durable mirror but has no configured
        // current staff authority. Historical ghost state must not grant hiding.
        use sqlx::ConnectOptions;
        let options = h
            .pool
            .connect_options()
            .as_ref()
            .clone()
            .options([("role", "oteryn_game_runtime")]);
        let runtime_url = options.to_url_lossy();
        let player_root =
            crate::durability::DurabilityRoot::connect_test_runtime(runtime_url.as_str())?;
        assert!(player_root.maintain_ready_once().await?);
        player_root.configure_staff(None).await?;
        let player_authority = player_root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let status = player_root
            .staff_live_status(&player_authority, owner.holder, fence)
            .await?;
        assert_eq!(status.group, 0);
        assert!(!status.ghost && !status.invisible);
        assert!(
            !player_root
                .staff_tools_actor_authorized(&player_authority, owner.holder, fence, "operator", 4)
                .await?
        );
        drop(owner);
        drop(authority);
        drop(player_authority);
        drop(player_root);
        drop(seal);
        h.cleanup().await
    })
}
