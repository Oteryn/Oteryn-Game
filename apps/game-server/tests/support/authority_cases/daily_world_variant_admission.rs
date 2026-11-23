use crate::bestiary_postgres_harness as fixture;
use crate::durability::character_last_position::{
    FinalLastPosition, LastPositionCell, LastPositionRecord,
};
use crate::durability::world_event::changes::{
    finalize_reset_for_test,
    variants::{PassageStep, SPIDER_BOSS, SpiderDeath},
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_world_change::variants::Selected;

#[test]
fn daily_world_variant_admission_follows_the_committed_reset() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    pg(admin, "daily_variant_admission")
}

#[test]
fn daily_world_variant_warpath_early_reentry_restarts_the_cooldown() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    pg(admin, "daily_variant_warpath")
}

#[test]
fn daily_world_variant_spider_visits_and_credited_deaths_survive_replay_and_reset()
-> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    pg(admin, "daily_variant_spider")
}

fn pg(admin: String, case: &'static str) -> fixture::TestResult {
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let h = fixture::Harness::create(admin, case, true).await?;
                    let result = match case {
                        "daily_variant_admission" => admission(&h).await,
                        "daily_variant_warpath" => warpath(&h).await,
                        "daily_variant_spider" => spider(&h).await,
                        _ => Err("unknown daily variant test".into()),
                    };
                    h.cleanup().await?;
                    result
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "daily variant admission thread failed")??;
    eprintln!("DAILY_VARIANT_POSTGRES_CASE_EXECUTED {case} elapsed_ms={}", started.elapsed().as_millis());
    Ok(())
}

async fn reset(h: &fixture::Harness, epoch: u64) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'1970-01-01T06:00:00Z','1970-01')")
        .bind(world.as_bytes().as_slice())
        .bind(epoch.to_string())
        .bind([7_u8; 32].as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
        .bind(world.as_bytes().as_slice())
        .bind(epoch.to_string())
        .execute(&mut *tx)
        .await?;
    finalize_reset_for_test(&mut tx, world, epoch).await?;
    tx.commit().await?;
    Ok(())
}

async fn admission(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    for epoch in [1, 2] {
        reset(h, epoch).await?;
        let day = h.root.world_changes(world, epoch).await?.ok_or("current day")?;
        assert_eq!(day.variants, Selected::roll(world.as_bytes(), epoch)?);
        assert!(h.root.world_change_active(world, "spirit_grounds").await?);
        assert_eq!(h.root.world_changes(world, epoch).await?, Some(day));
        assert_eq!(h.root.world_changes(world, epoch - 1).await?, None);
    }
    Ok(())
}

fn choose(
    first: u64,
    family: &str,
    variant: &str,
) -> fixture::TestResult<u64> {
    let world = fixture::scope()?.world_id();
    for epoch in first..first+1_000 {
        if Selected::roll(world.as_bytes(), epoch)?.active().any(|(d, v)| d.key == family && v.key == variant) {
            return Ok(epoch);
        }
    }
    Err("no occurrence within test range".into())
}

async fn make_session_current(h: &fixture::Harness) -> fixture::TestResult {
    sqlx::query("INSERT INTO game_durability_fresh_admission_receipts SELECT '\\x01'::bytea || decode(repeat('00',32),'hex'),game_session_id,account_id,character_id,world_id,runtime_scope_channel_id,character_lease_generation,scope_ownership_generation,1,current_transport_ref,1,'{}',1 FROM game_durability_reconnect_sessions ON CONFLICT DO NOTHING")
        .execute(&h.pool).await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2,control_loss_epoch=NULL,original_grace_deadline=NULL,predecessor_generation=NULL,fresh_replay_key='\\x01'::bytea || decode(repeat('00',32),'hex')")
        .execute(&h.pool).await?;
    Ok(())
}

async fn passage(
    h: &fixture::Harness,
    epoch: u64,
    position: (u16, u16, i8),
    command: u8,
) -> fixture::TestResult<Option<(u16, u16, i8)>> {
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(fixture::debug)?;
    let fence = fixture::fence(1)?.item_fence();
    let world_id = fence.runtime_scope.world_id();
    let (x, y, floor) = Selected::roll(world_id.as_bytes(), epoch)?
        .passage(position)
        .map_or(position, |(_, portal)| portal.destination);
    let step = PassageStep {
        epoch,
        position,
        command: CommandRef::new(fence.game_session_id, CommandId::new(u64::from(command)).map_err(fixture::debug)?),
        binding: [command; 32],
        arrival: FinalLastPosition {
            record: LastPositionRecord {
                world_id,
                cell: LastPositionCell { x: i32::from(x), y: i32::from(y), floor: i16::from(floor) },
                coordinate_frame_digest: [7; 32],
                map_revision_digest: [7; 32],
                server_artifact_digest: [7; 32],
                vitals: None,
                rest: None,
            },
            write_sequence: u64::from(command),
        },
    };
    h.root.daily_variant_passage(&authority, &h.node, fence, step).await.map_err(|e| fixture::debug(e).into())
}

async fn warpath(h: &fixture::Harness) -> fixture::TestResult {
    let epoch = choose(1, "warpath", "east_femor_hills")?;
    reset(h, epoch).await?;
    make_session_current(h).await?;
    let entry = (32617, 31828, 7);
    assert_eq!(passage(h, epoch, entry, 1).await?, Some((32619, 31828, 7)));
    let saved: (i32, i32, i16) = sqlx::query_as("SELECT tile_x,tile_y,tile_floor FROM game_character_last_positions")
        .fetch_one(&h.pool).await?;
    assert_eq!(saved, (32619, 31828, 7));
    sqlx::query("UPDATE game_daily_variant_warpath SET last_entry_attempt_ms=last_entry_attempt_ms-10000")
        .execute(&h.pool).await?;
    let timestamp = || sqlx::query_scalar::<_, i64>("SELECT last_entry_attempt_ms FROM game_daily_variant_warpath").fetch_one(&h.pool);
    let before = timestamp().await?;
    assert_eq!(passage(h, epoch, entry, 2).await?, None);
    let after = timestamp().await?;
    assert!(after > before);
    assert_eq!(passage(h, epoch, entry, 2).await?, None);
    assert_eq!(timestamp().await?, after);
    assert_eq!(passage(h, epoch, (32619, 31827, 7), 3).await?, Some((32617, 31827, 7)));
    sqlx::query("UPDATE game_daily_variant_warpath SET last_entry_attempt_ms=last_entry_attempt_ms-86401000")
        .execute(&h.pool).await?;
    assert_eq!(passage(h, epoch, entry, 4).await?, Some((32619, 31828, 7)));
    Ok(())
}

async fn spider(h: &fixture::Harness) -> fixture::TestResult {
    use crate::foundation::{ChannelContentPin, ChannelId, ChannelRuntimeV1, NodeId};
    let fence = fixture::fence(1)?;
    let world = fence.runtime_scope.world_id();
    let first = choose(1, "spider_nest", "open")?;
    let second = choose(first+1, "spider_nest", "open")?;
    reset(h, first).await?;
    make_session_current(h).await?;
    let entry = (32817, 32280, 7);
    let landing = (32816, 32280, 8);
    assert_eq!(passage(h, first, entry, 1).await?, Some(landing));
    assert_eq!(passage(h, first, entry, 1).await?, Some(landing));
    let channel = ChannelId::decode(&fixture::id(fixture::CHANNEL)).map_err(fixture::debug)?;
    let node = NodeId::decode(&fixture::id(1)).map_err(fixture::debug)?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world, channel, node, 1, 1, 1, "runtime-scope-assignment:1", 16,
        ChannelContentPin::from_activation(world, 1, [7;32], [7;32], [7;32], [7;32], (32817,32280,8)),
    ).map_err(fixture::debug)?;
    // The same native home respawns, so every death has a distinct sealed incarnation.
    runtime.begin_activation_spawn_sources(vec![(
        vec![((32817, 32280, 8), 90_000, SPIDER_BOSS.to_owned(), 1800, false)], 5_000,
    )]).map_err(fixture::debug)?;
    runtime.realize_activation_spawn_window(0).map_err(fixture::debug)?;
    runtime.bind_world_spawn_source(0, "spider_nest", first).map_err(fixture::debug)?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(fixture::debug)?;
    for ordinal in 0..3_u64 {
        let epoch = if ordinal == 0 { first } else { second };
        if ordinal == 1 {
            reset(h, second).await?;
            assert_eq!(passage(h, first, entry, 2).await?, None);
            assert_eq!(passage(h, second, entry, 3).await?, Some(landing));
            runtime.project_world_spawn_points(|base, _| {
                Some(oteryn_gameplay_entities::carrier::WorldSpawnProjection {
                    change_key: "spider_nest".to_owned(), generation: second,
                    creature: Some((base.to_owned(), 1800, false)),
                })
            }).map_err(fixture::debug)?;
        }
        if ordinal == 1 {
            crate::gameplay_transport::drive_owner_spawns(&mut runtime, 150_000_000);
            crate::gameplay_transport::drive_owner_spawns(&mut runtime, 155_000_000);
        }
        let (point, _, actor) = runtime.spawn_point_creatures()[0];
        let actor = actor.ok_or("native spider actor")?;
        runtime.commit_monster_lab_damage(actor, b"spider-lethal", 1800).map_err(fixture::debug)?;
        let mut owner = runtime.borrow_combat_death();
        let receipt = owner.committed_lethal_receipt(actor).map_err(fixture::debug)?;
        owner.project_committed_lethal(receipt).map_err(fixture::debug)?;
        let principal = crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
            .map_err(fixture::debug)?;
        let death = SpiderDeath::capture(&mut runtime, actor, principal, epoch)
            .ok_or("sealed spider home death")?;
        for _ in 0..2 {
            h.root.record_spider_nest_death(&authority, &h.node, fence.runtime_scope, fence.scope_ownership_generation, death).await?;
        }
        let now = ordinal * 200_000_000;
        crate::gameplay_transport::drive_owner_spawns(&mut runtime, now);
        let due = runtime.pending_respawn(point).ok_or("spider respawn")?.due_micros;
        crate::gameplay_transport::drive_owner_spawns(&mut runtime, due);
        crate::gameplay_transport::drive_owner_spawns(&mut runtime, due + 5_000_000);
    }
    let progress: Vec<(i64, i16)> = sqlx::query_as("SELECT visits,deaths FROM game_daily_variant_spider_progress ORDER BY reset_epoch")
        .fetch_all(&h.pool).await?;
    assert_eq!(progress, [(1, 1), (1, 2)]);
    assert_eq!(passage(h, second, entry, 4).await?, None);
    assert_eq!(passage(h, second, (32818, 32280, 8), 5).await?, Some((32818, 32280, 7)));
    Ok(())
}
