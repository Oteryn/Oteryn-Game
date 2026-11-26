//! W17: exercise item retirement through the arena match wire owner.
use super::arena_match_fixtures::fixture;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::map::{boot, overlay};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::arena::{self as wire, ArenaAction, ArenaDisposition};

#[test]
fn arena_reset_proof_preserves_runtime_ground_removal_guard() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        let h = Harness::create(admin, "arena_removal_acl", true).await?;
        eprintln!("arenapkg-marker: arena_reset_proof_preserves_runtime_ground_removal_guard db ready");
        sqlx::raw_sql(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/game-server/migrations/3272_arena_reset_retire_cause.sql"
        )))
        .execute(&h.pool)
        .await?;
        super::item_look_fixtures::seed_main_backpack(&h, &[("oteryn:item.tibia.i3031", 1)])
            .await?;
        let mut tx = h.pool.begin().await?;
        // Seed a plain Ground row, then exercise the real deferred deletion
        // trigger as the role used by the runtime.
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(71).as_slice())
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO game_item_ground_locations(item_instance_id, world_id, channel_id,
                runtime_scope_ownership_generation, spatial_position, corpse_ref, map_revision,
                content_revision, native_room_placement_context, stack_ordinal)
             VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,
                1,decode('00000001000000010007','hex'),decode('01','hex'),'map','content',
                decode('01','hex'),1)",
        )
        .bind(id(71).as_slice())
        .bind(id(42).as_slice())
        .bind(id(43).as_slice())
        .execute(&mut *tx)
        .await?;
        sqlx::query("SET LOCAL session_replication_role=origin")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(71).as_slice())
            .execute(&mut *tx)
            .await?;
        let error = sqlx::query("SET CONSTRAINTS game_item_ground_removal_proven IMMEDIATE")
            .execute(&mut *tx)
            .await
            .err()
            .ok_or("the ordinary proof must reject a deletion without an owner receipt")?;
        assert_eq!(
            error.as_database_error().and_then(|error| error.code()).as_deref(),
            Some("23514")
        );
        tx.rollback().await?;
        h.cleanup().await
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResetCase {
    Top,
    Container,
    CorpseLoot,
    NativeFloor,
    ProjectionRetry,
    FreshStatus,
    FreshLeave,
}

#[test]
fn arena_fresh_status_while_reset_refused_returns_busy() -> TestResult {
    run_reset_case(
        ResetCase::FreshStatus,
        "arena_fresh_status_while_reset_refused_returns_busy",
    )
}

#[test]
fn arena_leave_while_reset_refused_cancels_pending_entry() -> TestResult {
    run_reset_case(
        ResetCase::FreshLeave,
        "arena_leave_while_reset_refused_cancels_pending_entry",
    )
}

#[test]
fn arena_wire_entry_retires_corpse_and_loot() -> TestResult {
    run_reset_case(ResetCase::CorpseLoot, "arena_wire_entry_retires_corpse_and_loot")
}

#[test]
fn arena_wire_entry_retires_ground_with_positive_storage_floor() -> TestResult {
    run_reset_case(
        ResetCase::NativeFloor,
        "arena_wire_entry_retires_ground_with_positive_storage_floor",
    )
}

#[test]
fn svargrond_wire_entry_retires_the_populated_ground_tree() -> TestResult {
    run_reset_case(ResetCase::Container, "svargrond_wire_entry_retires_the_populated_ground_tree")
}

#[test]
fn svargrond_poll_projects_committed_wire_entry_after_transient_ground_refusal() -> TestResult {
    run_reset_case(ResetCase::ProjectionRetry, "svargrond_poll_projects_committed_wire_entry_after_transient_ground_refusal")
}

#[test]
fn svargrond_pit_reset_retires_only_the_movable_top_item_under_current_lease() -> TestResult {
    run_reset_case(ResetCase::Top, "svargrond_pit_reset_retires_only_the_movable_top_item_under_current_lease")
}

fn run_reset_case(case: ResetCase, test: &'static str) -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case, test).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "arena item reset scenario panicked")??;
    eprintln!(
        "W17 POSTGRES_CASE_COMPLETED duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario(case: ResetCase, test: &'static str) -> TestResult {
    run(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, "arena_pit_item_reset", false, false)
            .await?;
        eprintln!("arenapkg-marker: {test} db ready");
        super::reset();
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let active = controller.active().ok_or("active content")?;
        let native = active.native_gameplay().ok_or("native content")?;
        let (bytes, pins) = fixture::bundle_with_pillars(native)?;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let world = boot::boot(
            &bytes,
            &boot::BootPins {
                bundle: pins,
                map_revision: overlay::map_revision(&base),
                start: overlay::TilePos {
                    x: 32251,
                    y: 31098,
                    floor: -6,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                Some(crate::map::facts::ItemDefinition {
                    reference: native
                        .item_index()?
                        .definition_ref("Item", key, "definition-r1")?,
                    solid: Some(key.ends_with(".i1841")),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        fixture::runtime(&mut deps, active, &pin, &world)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        {
            let runtime = deps.runtime.lock().await;
            owner
                .states
                .spell_states
                .lock()
                .await
                .initialize(
                    &runtime,
                    deps.actor,
                    deps.admitted.game_session_id,
                    crate::spell::cast::CharacterCastFacts {
                        vocation: crate::spell::Vocation::Knight,
                        magic_level: 0,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now(),
                )
                .ok_or("player spell initialization")?;
        }
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 17, 170],
        )
        .ok_or("capabilities")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice())
            .bind(world.frame_binding_digest(deps.admitted.world_id).as_slice())
            .execute(&h.pool).await?;
        super::item_look_fixtures::seed_main_backpack(&h, &[("oteryn:item.tibia.i3043", 2)])
            .await?;
        owner.admit_progression_session(&mut admitted).await;
        let fence = admitted.item_fence.ok_or("item fence")?;

        for (n, item, x) in [
            (1, "oteryn:item.tibia.i3031", 32207),
            (2, "oteryn:item.tibia.i3035", 32207),
            (3, "oteryn:item.tibia.i3031", 32208),
            (4, "oteryn:item.tibia.i3031", 32215),
        ] {
            super::quest_world_callbacks_fixtures::input_item(
                &owner,
                fence,
                &Offering {
                    position: Position {
                        x,
                        y: 31098,
                        floor: -7,
                    },
                    item: item.into(),
                    count: 1,
                },
                n,
                false,
            )
            .await?;
        }
        let (entry_table, child_count) = if case == ResetCase::CorpseLoot {
            (
                "game_item_corpse_container_entries",
                i64::from(crate::durability::item_mint::CORPSE_CONTAINER_ENTRIES_MAX),
            )
        } else {
            ("game_item_nested_entries", 1)
        };
        let tree = if matches!(case, ResetCase::Container | ResetCase::CorpseLoot) {
            super::quest_world_callbacks_fixtures::input_item(
                &owner, fence,
                &Offering {
                    position: Position { x: 32207, y: 31098, floor: -7 },
                    item: "oteryn:item.tibia.i2854".into(),
                    count: 1,
                },
                5, false,
            ).await?;
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            let root: String = sqlx::query_scalar(
                "SELECT i.item_instance_id::text FROM game_item_instances i
                 JOIN game_item_ground_locations g USING(item_instance_id)
                 WHERE i.definition_production_key='oteryn:item.tibia.i2854'"
            ).fetch_one(&mut *tx).await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "WITH child AS (
                    INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,
                      definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
                    SELECT game_character_uuid_v7(),world_id,'Item','oteryn:item.tibia.i3031',
                       'definition-r1',3,1,game_character_uuid_v7()
                    FROM game_item_instances CROSS JOIN generate_series(1, $2::bigint)
                    WHERE item_instance_id=$1::uuid
                    RETURNING item_instance_id, world_id
                 )
                 INSERT INTO {entry_table}(item_instance_id,world_id,parent_item_instance_id,
                    placement_ordinal,placed_transaction_id)
                 SELECT item_instance_id,world_id,$1::uuid,row_number() OVER (ORDER BY item_instance_id),
                    game_character_uuid_v7() FROM child"
            ))).bind(&root).bind(child_count).execute(&mut *tx).await?;
            tx.commit().await?;
            Some(root)
        } else {
            None
        };
        owner.states.loop_ground.invalidate();
        let creature = deps
            .runtime
            .lock()
            .await
            .spawn_arena_opponent(
                "oteryn:creature.frostfur",
                crate::foundation::MovementLocalPosition {
                    x: 32208,
                    y: 31098,
                    floor: 7,
                },
                |runtime, at| {
                    Some(crate::gameplay_transport::arena_match::walkable(
                        &owner, runtime, at,
                    ))
                },
            )
            .map_err(debug)?;

        assert_eq!(ground_roots(&h).await?, 4);
        let status = wire::encode_match_request(ArenaAction::Status).map_err(debug)?;
        let status_id = admitted.continuity.next_command_id;
        let (_, status_reply) = super::guild_wire_fixtures::exchange(
            &owner,
            &mut admitted,
            status_id,
            wire::COMMAND_TYPE_ARENA_MATCH,
            &status,
        )
        .await?;
        assert_eq!(
            wire::decode_match_result(&status_reply)
                .map_err(debug)?
                .state
                .active_rank,
            0
        );
        assert_eq!(ground_roots(&h).await?, 4, "no pit lease yet");

        if matches!(case, ResetCase::FreshStatus | ResetCase::FreshLeave) {
            fresh_command_while_reset_refused(&h, &owner, admitted, case).await?;
            drop(owner);
            drop(deps);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }

        let payload = wire::encode_match_request(ArenaAction::Enter).map_err(debug)?;
        let entry = admitted.continuity.next_command_id;
        if case == ResetCase::ProjectionRetry {
            sqlx::raw_sql(
                "CREATE FUNCTION refuse_test_pit_reset() RETURNS trigger LANGUAGE plpgsql AS
                   $$ BEGIN RAISE EXCEPTION 'test transient pit reset refusal'; END $$;
                 CREATE TRIGGER refuse_test_pit_reset BEFORE INSERT ON game_arena_pit_item_resets
                   FOR EACH ROW EXECUTE FUNCTION refuse_test_pit_reset();"
            ).execute(&h.pool).await?;
            let before = deps.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?;
            let original_admission = admitted;
            let sent = super::guild_wire_fixtures::exchange(
                &owner, &mut admitted, entry, wire::COMMAND_TYPE_ARENA_MATCH, &payload,
            ).await;
            let replay = owner.arena_match_wire(original_admission, entry, &payload).await;
            let (status, bytes) = sent.map_err(|error| format!(
                "committed Enter lost its wire reply during Ground refusal: {error}; replay replied: {}",
                replay.is_some()
            ))?;
            assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
            let result = wire::decode_match_result(&bytes).map_err(debug)?;
            assert_eq!(result.disposition, ArenaDisposition::Applied);
            assert_eq!(result.state.pit, 1);
            let replay = replay.ok_or("committed Enter replay returned no reply during Ground refusal")?;
            assert_eq!(replay, bytes);
            assert_eq!(deps.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?, before);
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_svargrond_pit_leases").fetch_one(&h.pool).await?, 1);
            sqlx::query("DROP TRIGGER refuse_test_pit_reset ON game_arena_pit_item_resets")
                .execute(&h.pool).await?;
            owner.drain_monster_melee().await;
            assert_ne!(deps.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?, before,
                "native polling must project a committed entry after the reset becomes available");
            let replay = owner.arena_match_wire(admitted, entry, &payload).await.ok_or("entry replay")?;
            assert_eq!(replay, bytes);
            assert_eq!(ground_roots(&h).await?, 3);
            return Ok(());
        }
        if let Some(tree) = tree {
            let reply = owner.arena_match_wire(admitted, entry, &payload).await;
            let retired: (i16, i64, i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT i.lifecycle,
                 (SELECT count(*) FROM game_item_decay_retire_receipts WHERE ground_root_item_instance_id=i.item_instance_id AND cause_kind='ARENA_RESET'),
                 (SELECT count(*) FROM {entry_table} WHERE parent_item_instance_id=i.item_instance_id),
                 (SELECT count(*) FROM game_item_audit_outbox o JOIN game_item_decay_retire_receipts r USING(event_id)
                    WHERE r.ground_root_item_instance_id=i.item_instance_id)
                 FROM game_item_instances i WHERE i.item_instance_id=$1::uuid"
            ))).bind(&tree).fetch_one(&h.pool).await?;
            assert_eq!(
                retired,
                (2, child_count + 1, 0, child_count + 1),
                "the root and every child retire together"
            );
            let bytes = reply.ok_or("entry response")?;
            assert_eq!(wire::decode_match_result(&bytes).map_err(debug)?.disposition, ArenaDisposition::Applied);
            let replay = owner.arena_match_wire(admitted, entry, &payload).await.ok_or("entry replay")?;
            assert_eq!(replay, bytes);
            assert_eq!(ground_roots(&h).await?, 4, "only the exposed tree retires");
            return Ok(());
        }
        let index = native.item_index().ok_or("native item index")?;
        let barrier = std::sync::Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
        let enter = super::guild_wire_fixtures::exchange(
            &owner,
            &mut admitted,
            entry,
            wire::COMMAND_TYPE_ARENA_MATCH,
            &payload,
        );
        let rebuild = async {
            tokio::time::timeout(std::time::Duration::from_secs(5), barrier.0.notified()).await?;
            let result = async {
                let ground = crate::gameplay_transport::loop_move::loop_ground(
                    &owner, world.base(), Some(index),
                ).await.ok_or("concurrent Ground rebuild")?;
                assert_eq!(ground_roots(&h).await?, 4, "rebuild sees the pre-reset Ground");
                if case == ResetCase::NativeFloor {
                    // The reset has already observed map coordinates. Persist the
                    // corresponding native Ground encoding before its locked read.
                    let mut tx = h.pool.begin().await?;
                    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                    let encoded = sqlx::query(
                        "UPDATE game_item_ground_locations g
                         SET spatial_position=substring(g.spatial_position for 8)||int2send(7::smallint)
                         FROM game_item_instances i
                         WHERE g.item_instance_id=i.item_instance_id
                           AND i.definition_production_key='oteryn:item.tibia.i3035'",
                    ).execute(&mut *tx).await?;
                    assert_eq!(encoded.rows_affected(), 1);
                    tx.commit().await?;
                }
                Ok::<_, Box<dyn std::error::Error>>(ground)
            }.await;
            barrier.1.notify_one();
            result
        };
        let (entered, rebuilt) = tokio::join!(
            crate::gameplay_transport::arena_match::ARENA_ITEM_RESET_BARRIER.scope(
                std::sync::Arc::clone(&barrier),
                enter,
            ),
            rebuild,
        );
        let (_, bytes) = entered?;
        let rebuilt = rebuilt?;
        let result = wire::decode_match_result(&bytes).map_err(debug)?;
        assert_eq!(result.disposition, ArenaDisposition::Applied);
        assert_eq!(result.state.pit, 1);
        assert!(!deps.runtime.lock().await.contains_live_creature(creature));
        assert_eq!(
            ground_roots(&h).await?,
            3,
            "only the exposed top root retires"
        );
        let after = crate::gameplay_transport::loop_move::loop_ground(
            &owner, world.base(), Some(index),
        ).await.ok_or("post-reset Ground rebuild")?;
        assert!(
            !std::sync::Arc::ptr_eq(&rebuilt, &after),
            "the committed reset invalidates the concurrent pre-reset projection",
        );
        let envelope: Vec<u8> = sqlx::query_scalar(
            "SELECT o.envelope FROM game_item_decay_retire_receipts r JOIN game_item_audit_outbox o USING(event_id)
             WHERE r.cause_kind='ARENA_RESET'",
        ).fetch_one(&h.pool).await?;
        let (_, retired) =
            crate::durability::item_decay_retire_audit::decode_decay_retire_envelope(&envelope)
                .map_err(debug)?;
        let cause = retired.arena_reset.ok_or("arena audit cause")?;
        assert_eq!(cause.game_session_id, admitted.game_session_id.as_bytes());
        assert_eq!(cause.pit, 1);
        assert_eq!(
            retired
                .before
                .ok_or("before")?
                .definition
                .ok_or("definition")?
                .production_key,
            "oteryn:item.tibia.i3035"
        );
        assert!(sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM game_svargrond_pit_leases
             WHERE run_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND pit=$3 AND expires_at_ms=$4)",
        ).bind(&cause.run_id).bind(&cause.game_session_id).bind(i16::try_from(cause.pit)?)
            .bind(i64::try_from(cause.deadline_unix_ms)?).fetch_one(&h.pool).await?);
        let replay = owner
            .arena_match_wire(admitted, entry, &payload)
            .await
            .ok_or("entry replay")?;
        assert_eq!(replay, bytes);
        assert_eq!(
            ground_roots(&h).await?,
            3,
            "replay cannot peel another layer"
        );
        Ok(())
    })
}

async fn fresh_command_while_reset_refused(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    case: ResetCase,
) -> TestResult {
    use crate::gameplay_transport::connection::{IDLE_LIVENESS, serve_admitted};
    use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
    use oteryn_protocol_oteryn::{
        ClientCommandValue, CommandStatus, MessageType, decode_command_result, decode_wire_envelope,
        encode_client_command,
    };

    let mut ground_lock = h.pool.begin().await?;
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))",
    )
    .bind(admitted.world_id.as_bytes().as_slice())
    .bind(admitted.channel_id.as_bytes().as_slice())
    .execute(&mut *ground_lock)
    .await?;

    let (mut server, mut client) = tokio::io::duplex(1024);
    let serving = async move { serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await };
    let driving = async move {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let mut send = async |offset, action, expected| -> TestResult<wire::ArenaResult> {
            let command_id = admitted.continuity.next_command_id + offset;
            let payload = wire::encode_match_request(action).map_err(debug)?;
            let frame = encode_client_command(
                admitted.continuity.connection_generation,
                &ClientCommandValue {
                    command_id,
                    command_type: wire::COMMAND_TYPE_ARENA_MATCH,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            write_frame(&mut client, &frame).await?;
            loop {
                let frame = read_frame(&mut client).await.map_err(|error| {
                    format!("expected {expected:?} for arena {action:?} on the same connection: {error}")
                })?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let reply = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(reply.command_id, command_id);
                    assert_eq!(reply.status, CommandStatus::Accepted);
                    let arena = wire::decode_match_result(reply.payload).map_err(debug)?;
                    assert_eq!(arena.disposition, expected);
                    return Ok(arena);
                }
            }
        };
        let roots = ground_roots(h).await?;
        let entered = send(0, ArenaAction::Enter, ArenaDisposition::Applied).await?;
        assert_eq!(entered.state.pit, 1);
        assert_eq!(ground_roots(h).await?, roots, "Ground is still locked");
        if case == ResetCase::FreshLeave {
            let leases = "SELECT count(*) FROM game_svargrond_pit_leases";
            assert_eq!(sqlx::query_scalar::<_, i64>(leases).fetch_one(&h.pool).await?, 1);
            let left = send(1, ArenaAction::Leave, ArenaDisposition::Applied).await?;
            assert_eq!((left.state.active_rank, left.state.pit), (0, 0));
            assert_eq!(sqlx::query_scalar::<_, i64>(leases).fetch_one(&h.pool).await?, 0);
            ground_lock.rollback().await?;
            let status = send(2, ArenaAction::Status, ArenaDisposition::Applied).await?;
            assert_eq!((status.state.active_rank, status.state.pit), (0, 0));
            assert_eq!(ground_roots(h).await?, roots, "Leave cancelled the pending reset");
        } else {
            let busy = send(1, ArenaAction::Status, ArenaDisposition::Busy).await?;
            assert_eq!(busy.state.pit, 1);
            assert_eq!(ground_roots(h).await?, roots, "Busy leaves the reset pending");
            ground_lock.rollback().await?;
            let status = send(2, ArenaAction::Status, ArenaDisposition::Applied).await?;
            assert_eq!(status.state.pit, 1);
            assert_eq!(ground_roots(h).await?, roots - 1, "the pending reset succeeded");
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (_, result) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(serving, driving)
    })
    .await?;
    result
}

async fn ground_roots(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
         WHERE i.definition_production_key IN ('oteryn:item.tibia.i3031', 'oteryn:item.tibia.i3035')",
    ).fetch_one(&h.pool).await?)
}
