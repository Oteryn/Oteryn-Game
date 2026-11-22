use crate::bestiary_postgres_harness as fixture;
use oteryn_gameplay_world_change::daily::roll_calendar;

#[test]
fn fury_gate_daily_decision_persists_the_authored_entrance_and_return_city() -> fixture::TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async move {
                    let h =
                        fixture::Harness::create(admin, "fury_gate_daily_policy", false).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "Fury Gates PostgreSQL worker panicked")??;
    Ok(())
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    let first = roll_calendar(world.as_bytes(), 1, 6, 13);
    let second = (2_u64..=128)
        .map(|epoch| roll_calendar(world.as_bytes(), epoch, 6, 13))
        .find(|day| day.fury_exit != first.fury_exit)
        .ok_or("two reset epochs with different Fury Gate cities")?;
    for expected in [first, second] {
        let epoch = i64::try_from(expected.reset_epoch)?;
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT (world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch")
            .bind(world.as_bytes().as_slice()).bind(epoch).execute(&h.pool).await?;

        let day = h
            .root
            .world_changes(world, expected.reset_epoch)
            .await?
            .ok_or("active daily decision")?;
        assert_eq!(day.fury_gates, expected.fury_gates);
        assert_eq!(day.exit_layers(), expected.exit_layers());
        assert_eq!(
            h.root.world_changes(world, expected.reset_epoch).await?,
            Some(day)
        );
    }
    assert!(
        h.root
            .world_changes(world, first.reset_epoch)
            .await?
            .is_none()
    );
    Ok(())
}
