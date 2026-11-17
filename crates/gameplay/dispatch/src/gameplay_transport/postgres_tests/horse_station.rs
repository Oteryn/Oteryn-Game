use super::fixture::{Harness, debug, id, run};
use super::world_event::T94Bootstrap;
use super::{TestResult, reset};
use crate::foundation::{ChannelId, WorldId};

#[test]
fn horse_station_postgres_server_save_confines_expired_horses() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("horse-station", || {
        run(async |admin| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "horse_station", true, false).await?;
            let world = WorldId::decode(&id(42)).map_err(debug)?;
            let channel = ChannelId::decode(&id(43)).map_err(debug)?;
            let other_world = WorldId::decode(&id(44)).map_err(debug)?;
            let (_controller, pin) =
                crate::content::native_gameplay::test_support::activated_with_loot_tables(
                    world,
                    Some(include_bytes!(
                        "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                    )),
                );
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            h.root
                .t94_initialize(&authority, "oteryn:game:primary", &mut T94Bootstrap)
                .await?;
            h.root
                .t94_start_world(
                    &authority,
                    world,
                    &Default::default(),
                    "oteryn:game:primary",
                )
                .await?;
            sqlx::query("INSERT INTO game_horse_stations(world_id,loose_until_ms) VALUES(encode($1,'hex')::uuid,1),(encode($2,'hex')::uuid,1)")
                .bind(world.as_bytes().as_slice())
                .bind(other_world.as_bytes().as_slice())
                .execute(&h.pool)
                .await?;
            super::world_event::event_fixture::reset_epochs(
                &h,
                &authority,
                pin.qualified_room(),
                world,
                channel,
                1,
            )
            .await?;
            let deadline: Option<i64> = sqlx::query_scalar(
                "SELECT loose_until_ms FROM game_horse_stations WHERE world_id=encode($1,'hex')::uuid",
            )
            .bind(world.as_bytes().as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(deadline, None);
            let other_deadline: Option<i64> = sqlx::query_scalar(
                "SELECT loose_until_ms FROM game_horse_stations WHERE world_id=encode($1,'hex')::uuid",
            )
            .bind(other_world.as_bytes().as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(other_deadline, Some(1));
            Ok(())
        })
    })?;
    let elapsed = started.elapsed();
    assert!(elapsed > std::time::Duration::from_millis(10));
    println!(
        "PASS horse_station_postgres_server_save_confines_expired_horses elapsed_ms={}",
        elapsed.as_millis()
    );
    Ok(())
}
