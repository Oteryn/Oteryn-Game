//! Exercise World selection and migration 3359 through the fenced respawn owner.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::character_cyclopedia::CyclopediaWorldBinding;
use crate::durability::spell_owner_commit::SpellLanePermit;

#[test]
fn cyclopedia_rapid_respawn_world_save_is_atomic_and_replayable() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(|e| e.to_string())?
                .block_on(async {
                    let h =
                        fixture::Harness::create(admin, "cyclopedia_rapid_respawn", false).await?;
                    let result = verify(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Cyclopedia rapid respawn thread failed")??;
    Ok(())
}

#[test]
fn cyclopedia_rapid_respawn_migration_keeps_legacy_settlement_writable() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(|e| e.to_string())?
                .block_on(async {
                    let h = fixture::Harness::create(admin, "cyclopedia_legacy_settlement", false)
                        .await?;
                    let result = verify_legacy_settlement(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Cyclopedia legacy settlement thread failed")??;
    Ok(())
}

async fn verify_legacy_settlement(h: &fixture::Harness) -> fixture::TestResult {
    let applied: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version=3359 AND success)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert!(
        applied,
        "the production migrator must install 3359 before settlement"
    );

    let world = fixture::scope()?.world_id();
    let world = world.as_bytes().as_slice();
    let area = 1_i64;
    let epoch = "7";
    let mut seed = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *seed)
        .await?;
    sqlx::query(
        "INSERT INTO game_world_cyclopedia(world_id,bundle_digest,catalogue_digest,area_id,pool)
                 VALUES(encode($1,'hex')::uuid,$2,$3,$4,25000000)",
    )
    .bind(world)
    .bind([7_u8; 32].as_slice())
    .bind(oteryn_game_server::domain::cyclopedia::BasicArea::content_digest().as_slice())
    .bind(area)
    .execute(&mut *seed)
    .await?;
    seed.commit().await?;

    // Replay e2750951's first funded save as the old runtime role, including the deferred
    // guard at COMMIT. This is the statement shape in character_cyclopedia/storage.rs.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let pool: i64 = sqlx::query_scalar(
        "SELECT pool FROM game_world_cyclopedia WHERE world_id=encode($1,'hex')::uuid AND area_id=$2 FOR UPDATE",
    )
    .bind(world)
    .bind(area)
    .fetch_one(&mut *tx)
    .await?;
    let selected: Option<bool> = sqlx::query_scalar("SELECT improved FROM game_world_cyclopedia_epochs WHERE world_id=encode($1,'hex')::uuid AND epoch=$2::text::numeric AND area_id=$3")
        .bind(world).bind(epoch).bind(area).fetch_optional(&mut *tx).await?;
    assert_eq!(selected, None);
    sqlx::query("INSERT INTO game_world_cyclopedia_epochs(world_id,epoch,improved,donation_winner,pool_before,area_id) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$3,$4,$5)")
        .bind(world).bind(epoch).bind(true).bind(pool).bind(area).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_world_cyclopedia SET pool=$3 WHERE world_id=encode($1,'hex')::uuid AND area_id=$2")
        .bind(world).bind(area).bind(pool - 10_000_000).execute(&mut *tx).await?;
    tx.commit().await?;

    let saved: (bool, bool, i64, i64) = sqlx::query_as(
        "SELECT e.improved,e.donation_winner,e.pool_before,p.pool
         FROM game_world_cyclopedia_epochs e JOIN game_world_cyclopedia p USING(world_id,area_id)
         WHERE e.world_id=encode($1,'hex')::uuid AND e.epoch=$2::text::numeric AND e.area_id=$3",
    )
    .bind(world)
    .bind(epoch)
    .bind(area)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(saved, (true, true, 25_000_000, 15_000_000));
    Ok(())
}

async fn verify(h: &fixture::Harness) -> fixture::TestResult {
    let scope = fixture::scope()?;
    let world = scope.world_id();
    let world = world.as_bytes().as_slice();
    let bundle = [7; 32];
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    // Begin immediately before a deterministic random hit, with yesterday's winner
    // holding more gold than any of today's eligible areas.
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,
                    server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month)
                 SELECT encode($1,'hex')::uuid,epoch,$2,'fixture',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,now(),'2026-11'
                 FROM unnest(ARRAY[7,8,9]) e(epoch)")
        .bind(world).bind(bundle.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,7)")
        .bind(world)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_world_cyclopedia(world_id,bundle_digest,catalogue_digest,area_id,pool)
                 SELECT encode($1,'hex')::uuid,$2,$3,area,pool
                 FROM unnest(ARRAY[10000000,25000000,15000000,30000000]) WITH ORDINALITY AS a(pool,area)")
        .bind(world).bind(bundle.as_slice())
        .bind(oteryn_game_server::domain::cyclopedia::BasicArea::content_digest().as_slice())
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_cyclopedia_epochs(world_id,area_id,epoch,improved,donation_winner,pool_before)
                 VALUES(encode($1,'hex')::uuid,4,6,true,false,30000000)")
        .bind(world).execute(&mut *tx).await?;
    tx.commit().await?;
    eprintln!("POSTGRES_STARTED cyclopedia_rapid_respawn_world_save");

    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let permit = SpellLanePermit::of_fresh_scope(scope)
        .await
        .map_err(fixture::debug)?;
    let binding = CyclopediaWorldBinding {
        scope,
        generation: 1,
        bundle,
        epoch: 7,
        main_areas: [1, 2, 3, 4].into(),
        legacy_area: 1,
    };

    // Freeze the winning pool's old value so the epoch insert must roll back if
    // any debit is attempted.
    sqlx::query(
        "ALTER TABLE game_world_cyclopedia ADD CONSTRAINT reject_debit CHECK(area_id<>2 OR pool=25000000) NOT VALID",
    )
    .execute(&h.pool)
    .await?;
    assert!(
        h.root
            .prepare_cyclopedia_epoch(&authority, &h.node, &permit, binding.clone())
            .await
            .is_err()
    );
    let failed: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_world_cyclopedia_epochs WHERE epoch=7")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        failed, 0,
        "a failed winning-pool debit rolls back every epoch row"
    );
    sqlx::query("ALTER TABLE game_world_cyclopedia DROP CONSTRAINT reject_debit")
        .execute(&h.pool)
        .await?;
    h.root.await_ready().await?;

    for _ in 0..2 {
        let winners = h
            .root
            .prepare_cyclopedia_epoch(&authority, &h.node, &permit, binding.clone())
            .await?;
        assert_eq!(
            winners,
            [1, 2].into(),
            "one random area plus the highest eligible donor"
        );
    }
    let saved: Vec<(i64, bool, bool, i64, i64)> = sqlx::query_as(
        "SELECT e.area_id,e.improved,e.donation_winner,e.pool_before,p.pool
         FROM game_world_cyclopedia_epochs e JOIN game_world_cyclopedia p USING(world_id,area_id)
         WHERE e.epoch=7 ORDER BY e.area_id",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        saved,
        vec![
            (1, true, false, 10_000_000, 10_000_000),
            (2, true, true, 25_000_000, 0),
            (3, false, false, 15_000_000, 15_000_000),
            (4, false, false, 30_000_000, 30_000_000),
        ]
    );

    // Migration 3359 still rejects a debit that has no same-transaction winner receipt.
    let mut tx = h.pool.begin().await?;
    sqlx::query("UPDATE game_world_cyclopedia SET pool=0 WHERE area_id=3")
        .execute(&mut *tx)
        .await?;
    let error = tx
        .commit()
        .await
        .expect_err("the losing pool has no debit receipt");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|db| db.code())
            .as_deref(),
        Some("23514")
    );

    sqlx::query("UPDATE game_world_reset_active SET reset_epoch=8")
        .execute(&h.pool)
        .await?;
    assert!(
        h.root
            .prepare_cyclopedia_epoch(&authority, &h.node, &permit, binding.clone())
            .await
            .is_err()
    );
    h.root.await_ready().await?;
    let next = h
        .root
        .prepare_cyclopedia_epoch(
            &authority,
            &h.node,
            &permit,
            CyclopediaWorldBinding {
                epoch: 8,
                ..binding.clone()
            },
        )
        .await?;
    assert_eq!(
        next,
        [4].into(),
        "a previous winner can return after one day off"
    );
    // Retain a day partially written by an older binary, including its boost and
    // the unspent balance in a different area. Do not run a second selection.
    sqlx::query("INSERT INTO game_world_cyclopedia_epochs(world_id,area_id,epoch,improved,donation_winner,pool_before)
                 VALUES(encode($1,'hex')::uuid,1,9,true,false,10000000)")
        .bind(world).execute(&h.pool).await?;
    sqlx::query("UPDATE game_world_reset_active SET reset_epoch=9")
        .execute(&h.pool)
        .await?;
    let historical = CyclopediaWorldBinding {
        epoch: 9,
        ..binding
    };
    let frozen = h
        .root
        .prepare_cyclopedia_epoch(&authority, &h.node, &permit, historical.clone())
        .await?;
    assert_eq!(
        frozen,
        [1].into(),
        "complete an already selected day without another draw"
    );
    let loser_pool: i64 =
        sqlx::query_scalar("SELECT pool FROM game_world_cyclopedia WHERE area_id=3")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(loser_pool, 15_000_000);

    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_world_cyclopedia SET bundle_digest=$1 WHERE area_id=3")
        .bind([8_u8; 32].as_slice())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    assert!(
        matches!(
            h.root
                .prepare_cyclopedia_epoch(&authority, &h.node, &permit, historical)
                .await,
            Err(crate::durability::DurabilityError::InvalidStoredState)
        ),
        "even a settled epoch rejects a mismatched area pin"
    );
    Ok(())
}
