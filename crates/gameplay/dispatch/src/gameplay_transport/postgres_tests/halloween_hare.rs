//! An annual calendar observation must have both exact waves and a bound firing anchor.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::raid_schedule::RaidSchedulerCheck;
use crate::foundation::ScopeOwnershipGeneration;
use oteryn_gameplay_encounter_rules::raid::RaidCatalogue;

#[test]
fn w05d_halloween_hare_without_exact_waves_spawns_nothing() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            crate::bestiary_postgres_harness::runtime()
                .map_err(|error| error.to_string())?
                .block_on(async {
                    let h =
                        Harness::create_with_admission_seed(admin, "halloween_hare", true, false)
                            .await?;
                    let result = exercise(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Halloween Hare thread panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let world = deps.admitted.world_id;
    let channel = deps.admitted.channel_id;
    let bundle = [7; 32];
    let key = crate::content::ProductionKey::new("oteryn:raid.kazordoon_raids.the_halloween_hare")
        .map_err(debug)?;
    // A bound server-save schedule alone cannot activate a raid with missing waves.
    let mut missing_waves = serde_json::json!({
        "schema": "OTERYN_RAID_CATALOGUE/v1",
        "records": [{
            "key": key.as_str(),
            "kind": "calendar",
            "active": false,
            "inactive_reasons": ["waves_unknown"],
            "schedule": {
                "kind": "calendar",
                "window": {"start_month": 10, "start_day": 31, "end_month": 10, "end_day": 31},
                "anchor": "immediate_post_save",
                "repeat": false
            },
            "variant_group": null,
            "variants": [],
            "end": {"kind": "until_reset"}
        }]
    });
    let without_waves = RaidCatalogue::from_json(&missing_waves.to_string()).map_err(debug)?;
    let incomplete = without_waves.get(&key).ok_or("Hare without waves")?;
    assert!(incomplete.variants.is_empty());
    assert!(incomplete.select(bundle, id(81)).is_err());
    missing_waves["records"][0]["active"] = true.into();
    missing_waves["records"][0]["inactive_reasons"] = serde_json::json!([]);
    assert!(RaidCatalogue::from_json(&missing_waves.to_string()).is_err());
    assert_eq!(without_waves.active().count(), 0);

    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'hare-test-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,TIMESTAMPTZ '2026-10-31 06:00 Europe/Berlin','2026-10')")
        .bind(world.as_bytes().as_slice()).bind(bundle.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let now_ms = sqlx::query_scalar(
        "SELECT (extract(epoch FROM TIMESTAMPTZ '2026-10-31 06:00 Europe/Berlin') * 1000)::bigint",
    )
    .fetch_one(&h.pool)
    .await?;
    let request = RaidSchedulerCheck {
        world,
        channel,
        generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        reset_epoch: 1,
        now_ms,
        bundle_digest: bundle,
        candidates: vec![],
    };
    let outcome = h
        .root
        .check_raid_schedules(&authority, &h.node, request)
        .await?;
    assert!(outcome.firings.is_empty());
    let clock: i64 = sqlx::query_scalar("SELECT last_end_at_ms FROM game_world_raid_scheduler_clocks WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        clock, now_ms,
        "the fenced production scheduler reached its calendar check"
    );
    assert!(h.root.due_raid_firings(channel).await?.is_empty());
    let placements: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_world_raid_creature_placements")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(placements, 0);
    eprintln!("POSTGRES_CASE_COMPLETED W05d incomplete Hare wave refused");
    Ok(())
}
