//! Actual Summer settlement and staged corpse recovery on the shared owners.
use super::*;
use sqlx::Row;

#[path = "summer_handoff.rs"]
mod handoff;

pub(super) async fn qualify_summer_settlement(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    creature: &str,
    expected_xp: i64,
    children: i64,
    handoff: bool,
    target: crate::foundation::ExactActorRef,
) -> TestResult {
    let native = owner.active_generation.unwrap().native_gameplay().unwrap();
    let reward = native.reward_table().row(creature).map_err(debug)?;
    assert!(
        reward.race.is_some(),
        "the native Radiant has a Bestiary race"
    );
    let race = reward.race.as_ref().ok_or("Radiant Bestiary race")?;
    let kills: i64 = sqlx::query_scalar("SELECT kill_count FROM game_character_bestiary_progress WHERE character_id=encode($1,'hex')::uuid AND race_key=$2")
        .bind(id(41).as_slice()).bind(race.key()).fetch_one(&h.pool).await?;
    assert_eq!(
        kills, 1,
        "one durable kill for the exact Character and native race"
    );
    let corpse = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .next()
        .unwrap();
    let item = corpse.item.item_instance_id;
    assert_eq!(
        corpse.item.production_key,
        reward.corpse_item.production_key
    );
    let top: bool = sqlx::query_scalar("SELECT corpse_top_damage_character_id=encode($1,'hex')::uuid FROM game_item_mint_receipts WHERE item_instance_id=encode($2,'hex')::uuid")
        .bind(id(41).as_slice()).bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert!(top, "corpse ownership is the actual top-damage Character");
    let loot = sqlx::query("SELECT i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,e.parent_item_instance_id=encode($1,'hex')::uuid AS custody FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY e.placement_ordinal LIMIT 31")
        .bind(item.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(i64::try_from(loot.len())?, children);
    for row in loot {
        let key: String = row.try_get("definition_production_key")?;
        let family: String = row.try_get("definition_family")?;
        let revision: String = row.try_get("definition_revision_ref")?;
        let quantity: i64 = row.try_get("quantity")?;
        assert!(row.try_get::<bool, _>("custody")?);
        assert!(
            reward
                .loot_table
                .entries
                .iter()
                .any(|entry| entry.item.production_key == key
                    && entry.item.family == family
                    && entry.item.revision_ref == revision
                    && quantity >= i64::from(entry.min_count)
                    && quantity <= i64::from(entry.max_count)),
            "committed loot is a native table member: {key}"
        );
    }
    for sql in [
        "UPDATE game_spell_field_temporal_schedules SET duration_millis=1 WHERE item_instance_id=encode($1,'hex')::uuid",
        "DELETE FROM game_spell_field_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid",
        "INSERT INTO game_spell_field_temporal_schedules SELECT item_instance_id,4,blocks_movement,blocks_projectile,immovable_block_solid,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,target_family,target_production_key,target_revision,content_digest,created_xact_id FROM game_spell_field_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid AND source_stage=1",
    ] {
        let error = sqlx::query(sql)
            .bind(item.as_slice())
            .execute(&h.pool)
            .await
            .expect_err("immutable provenance and bounded stages");
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("23514")
        );
    }
    let schedules: Vec<(i32,i64,i64)> = sqlx::query_as("SELECT p.source_stage,p.duration_millis,p.expires_at_unix_ms-m.materialized_at FROM game_spell_field_temporal_schedules p JOIN game_item_mint_receipts m USING(item_instance_id) WHERE p.item_instance_id=encode($1,'hex')::uuid ORDER BY source_stage")
        .bind(item.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        schedules,
        vec![
            (1, 10_000, 10_000),
            (2, 300_000, 310_000),
            (3, 300_000, 610_000)
        ]
    );
    restart(h, "read")?;
    owner.drain_source_item_deadlines().await;
    assert_eq!(stage_count(h, item).await?, 0, "no early transform");
    owner.drain_corpse_deadlines().await;
    assert_eq!(owner.runtime.lock().await.visible_corpses().len(), 1);

    // Age the fixture's original clock as in corpse_decay::qualify. Production
    // writers, all transition proofs and commit-time clocks stay enabled.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let delta: i64 = sqlx::query_scalar("SELECT materialized_at-floor(extract(epoch FROM clock_timestamp())*1000)::bigint+611000 FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&mut *tx).await?;
    sqlx::query("UPDATE game_item_mint_receipts SET materialized_at=materialized_at-$2 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).bind(delta).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_spell_field_temporal_schedules SET expires_at_unix_ms=expires_at_unix_ms-$2 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).bind(delta).execute(&mut *tx).await?;
    tx.commit().await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .unwrap();
    // The server polls the corpse timer before the Item-stage timer. An
    // overdue untransformed corpse must not freeze a poisoned first-stage
    // retirement candidate. Temporarily advance/restore this fixture anchor.
    shift_materialization(h, item, -61_000).await?;
    owner.drain_corpse_deadlines().await;
    let reserved: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_decay_retire_reservations WHERE corpse_item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        reserved, 0,
        "stage catch-up precedes retirement reservation"
    );
    shift_materialization(h, item, 61_000).await?;
    restart(
        h,
        if handoff {
            "handoff-transforms"
        } else {
            "transforms"
        },
    )?;
    assert_eq!(
        stage_count(h, item).await?,
        3,
        "overdue recovery applies every exact stage"
    );
    let state: (String,String,i64) = sqlx::query_as("SELECT definition_production_key,state_revision::text,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    let terminal = match reward.corpse_item.production_key.as_str() {
        "oteryn:item.tibia.i54678" => "oteryn:item.tibia.i54681",
        "oteryn:item.tibia.i54407" => "oteryn:item.tibia.i54404",
        "oteryn:item.tibia.i54399" => "oteryn:item.tibia.i54396",
        "oteryn:item.tibia.i54415" => "oteryn:item.tibia.i54412",
        _ => return Err("unknown Summer corpse chain".into()),
    };
    assert_eq!(state.0, terminal);
    assert_eq!(state.1, "4");
    assert_eq!(state.2, 1);
    let live_children: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        live_children, children,
        "transforms preserve contents and custody"
    );
    owner.drain_corpse_deadlines().await;
    let projected = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .next()
        .ok_or("transformed live projection")?;
    assert_eq!(
        projected.item.production_key, state.0,
        "router refreshes the transformed corpse definition"
    );
    // A lost-response kill retry still names the original MINT definition.
    // It must reconcile that binding without reverting the transformed view.
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_corpse_item(
                target,
                item,
                &reward.corpse_item.family,
                &reward.corpse_item.production_key,
                &reward.corpse_item.revision_ref,
            )
            .map_err(debug)?;
        assert_eq!(runtime.visible_corpses()[0].item.production_key, terminal);
    }
    let generation = owner
        .runtime
        .lock()
        .await
        .owner_fence()
        .map_err(debug)?
        .generation();
    let fence = crate::durability::item_decay_retire::CorpseDecayFence {
        world_id: owner.world_id,
        channel_id: owner.channel_id,
        scope_ownership_generation: generation,
    };
    let early = h
        .root
        .freeze_decay_retire(
            owner.character,
            &h.node,
            fence,
            crate::durability::item_decay_retire::DecayRetireStep::corpse(item),
        )
        .await;
    if handoff {
        assert!(
            matches!(
                early,
                Err(crate::durability::item_decay_retire::DecayRetireError::AuthorityRejected)
            ),
            "the former owner is fenced out after restart"
        );
    } else {
        assert!(
            matches!(
                early,
                Err(
                    crate::durability::item_decay_retire::DecayRetireError::Refused(
                        crate::durability::item_decay_retire::DecayRetireRefusal::NotYetDue
                    )
                )
            ),
            "the terminal stage retains its own 60-second window"
        );
    }
    let audit: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_audit a USING(transaction_id) WHERE r.item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(audit, 3);
    // Advance only the fixture clock anchor through the terminal window.
    // Already committed stage schedules, receipts and audit bytes are unchanged.
    shift_materialization(h, item, -61_000).await?;
    restart(h, "retire")?;
    owner.drain_corpse_deadlines().await;
    let retired: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_decay_retire_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        retired,
        children + 1,
        "one retirement per Item, including contents"
    );
    for _ in 0..2 {
        assert_eq!(
            owner
                .drain_kill_rewards(GameSessionId::decode(&id(50)).map_err(debug)?)
                .await,
            DrainOutcome::Drained
        );
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(2))
            .unwrap();
        owner.drain_source_item_deadlines().await;
        owner.drain_corpse_deadlines().await;
    }
    assert_eq!(stage_count(h, item).await?, 3);
    assert_eq!(xp_receipts(h).await?, 1);
    let xp: i64 =
        sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(xp, expected_xp);
    assert!(owner.runtime.lock().await.visible_corpses().is_empty());
    Ok(())
}

async fn shift_materialization(h: &Harness, item: [u8; 16], delta: i64) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_mint_receipts SET materialized_at=materialized_at+$2 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).bind(delta).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn stage_count(h: &Harness, item: [u8; 16]) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_spell_field_temporal_receipts WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?)
}

// A distinct executable process reconstructs the recovery seal, database root,
// node proof and scope lane. Nothing from the parent's timer/queue is passed.
#[test]
fn summer_corpse_process_restart() -> TestResult {
    let Ok(database) = std::env::var("SUMMER_RESTART_DATABASE") else {
        return Ok(());
    };
    let parent = std::env::var("SUMMER_RESTART_PARENT")?;
    let mode = std::env::var("SUMMER_RESTART_MODE")?;
    run(async |admin| {
        let prefix = admin
            .strip_suffix("/postgres")
            .ok_or("test admin database")?;
        let url = format!("{prefix}/{database}");
        let root = crate::durability::DurabilityRoot::connect_test_runtime(&url)?;
        assert!(root.maintain_ready_once().await?);
        let retained = std::env::temp_dir()
            .join(format!("oteryn-revision-sequencer-parent-{parent}"))
            .join(&database);
        let recovery = crate::character_recovery_fence::CharacterRecoveryStore::open(
            retained,
            "character-primary",
            "game-ops",
        )
        .map_err(debug)?;
        let seal = recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let schedule = root
            .read_corpse_decay_schedule(&authority, world, channel)
            .await
            .map_err(debug)?;
        assert_eq!(schedule.corpses.len(), 1);
        let item = schedule.corpses[0].corpse_item_instance_id;
        let pool = sqlx::PgPool::connect(&url).await?;
        if mode == "transforms" || mode == "handoff-transforms" {
            let node = if mode == "handoff-transforms" {
                handoff::move_scope(&root, &pool, world, channel).await?
            } else {
                super::super::fixture::registered_node(&root, 1).await?
            };
            let generation = if mode == "handoff-transforms" { 2 } else { 1 };
            let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_lane(
                world, channel,
            )
            .await;
            assert_eq!(
                root.drain_spell_item_deadlines(
                    &permit,
                    &authority,
                    &node,
                    crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                    generation
                )
                .await
                .map_err(debug)?,
                3
            );
            assert_eq!(
                root.drain_spell_item_deadlines(
                    &permit,
                    &authority,
                    &node,
                    crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                    generation
                )
                .await
                .map_err(debug)?,
                0
            );
        } else if mode == "retire" {
            let generation: i32 = sqlx::query_scalar("SELECT ownership_generation::int FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).fetch_one(&pool).await?;
            let node =
                super::super::fixture::registered_node(&root, u8::try_from(generation)?).await?;
            let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_lane(
                world, channel,
            )
            .await;
            root.retire_decayed_corpse(
                &permit,
                &authority,
                &node,
                crate::durability::item_decay_retire::CorpseDecayFence {
                    world_id: world,
                    channel_id: channel,
                    scope_ownership_generation: crate::foundation::ScopeOwnershipGeneration::new(
                        u64::try_from(generation)?,
                    )
                    .map_err(debug)?,
                },
                item,
            )
            .await
            .map_err(debug)?;
            assert!(
                root.read_corpse_decay_schedule(&authority, world, channel)
                    .await
                    .map_err(debug)?
                    .corpses
                    .is_empty()
            );
        } else {
            assert_eq!(mode, "read");
            assert_eq!(
                schedule.corpses[0].decay_at_unix_ms - schedule.corpses[0].materialized_at_unix_ms,
                670_000
            );
            assert!(
                root.read_item_instance(&authority, item)
                    .await
                    .map_err(debug)?
                    .is_some()
            );
        }
        let kills: i64 = sqlx::query_scalar(
            "SELECT sum(kill_count)::bigint FROM game_character_bestiary_progress",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(kills, 1);
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
            .fetch_one(&pool)
            .await?;
        assert_eq!(receipts, 1);
        pool.close().await;
        Ok(())
    })
}

fn restart(h: &Harness, mode: &str) -> TestResult {
    let options = h.pool.connect_options();
    let database = options.get_database().ok_or("test database")?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "summer_corpse_process_restart",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("SUMMER_RESTART_DATABASE", database)
        .env("SUMMER_RESTART_PARENT", std::process::id().to_string())
        .env("SUMMER_RESTART_MODE", mode)
        .output()?;
    assert!(
        output.status.success(),
        "restart {mode}: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
