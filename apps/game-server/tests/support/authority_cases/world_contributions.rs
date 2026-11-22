//! Checks the same durable world-change gate used by paid NPC travel.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::world_event::changes::{self, ContributionWrite};
use oteryn_gameplay_world_change::owner::ContributionKind;

#[test]
fn world_contributions_steamship_gate_opens_after_shared_coal_and_save() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(|e| e.to_string())?
                .block_on(async {
                    let h = fixture::Harness::create(admin, "steamship_coal", false).await?;
                    let result = steamship_saves(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "steamship gate thread failed")??;
    Ok(())
}

async fn steamship_saves(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    let foreign = crate::foundation::WorldId::decode(&fixture::id(90)).map_err(fixture::debug)?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(fixture::debug)?;
    let recovery = authority.record_for(&h.root).map_err(fixture::debug)?;

    for epoch in 1..=10_i64 {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z'::timestamptz + $2 * interval '1 day','2031-06')")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind([1u8; 32].as_slice())
            .execute(&h.pool).await?;
    }
    for epoch in 1..=2 {
        activate(h, world, epoch).await?;
        assert!(!h.root.world_change_active(world, "steamship").await?);
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
            let id = [u8::try_from(epoch)?; 16];
            let amount = if epoch == 1 { 199 } else { 1 };
            assert_eq!(
                changes::contribute(&mut tx, &recovery, world, u64::try_from(epoch)?, id, ContributionKind::Coal, amount).await?,
                ContributionWrite::Applied(u64::from(amount))
            );
            assert_eq!(
                changes::contribute(&mut tx, &recovery, world, u64::try_from(epoch)?, id, ContributionKind::Coal, amount).await?,
                ContributionWrite::Replayed(u64::from(amount))
            );
            tx.commit().await?;
            assert!(!h.root.world_change_active(world, "steamship").await?);
    }
    activate(h, world, 3).await?;
    assert!(h.root.world_change_active(world, "steamship").await?);
    assert!(!h.root.world_change_active(foreign, "steamship").await?);
    for epoch in 4..=9 {
        activate(h, world, epoch).await?;
        assert!(h.root.world_change_active(world, "steamship").await?);
    }
    activate(h, world, 10).await?;
    assert!(!h.root.world_change_active(world, "steamship").await?);
    Ok(())
}

async fn activate(h: &fixture::Harness, world: crate::foundation::WorldId, epoch: i64) -> fixture::TestResult {
    sqlx::query("INSERT INTO game_world_reset_active(world_id,reset_epoch) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch")
        .bind(world.as_bytes().as_slice()).bind(epoch).execute(&h.pool).await?;
    assert!(h.root.world_changes(world, u64::try_from(epoch)?).await?.is_some());
    Ok(())
}
