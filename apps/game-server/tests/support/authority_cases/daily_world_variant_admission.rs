use crate::bestiary_postgres_harness as fixture;

#[test]
fn daily_world_variant_admission_follows_the_committed_reset() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let h = fixture::Harness::create(admin, "daily_variant_admission", false).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "daily variant admission thread failed")??;
    Ok(())
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    for epoch in [1_i64, 2] {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($2 * 86400),'1970-01')")
            .bind(world.as_bytes().as_slice())
            .bind(epoch)
            .bind([7_u8; 32].as_slice())
            .execute(&h.pool)
            .await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
            .bind(world.as_bytes().as_slice())
            .bind(epoch)
            .execute(&h.pool)
            .await?;
        let epoch = u64::try_from(epoch)?;
        let day = h.root.world_changes(world, epoch).await?.ok_or("current day")?;
        assert!(h.root.world_change_active(world, "spirit_grounds").await?);
        assert_eq!(h.root.world_changes(world, epoch).await?, Some(day));
        assert_eq!(h.root.world_changes(world, epoch - 1).await?, None);
    }
    Ok(())
}
