//! An annual calendar observation must have both exact waves and a bound firing anchor.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::raid_schedule::{
    CalendarRaidScheduleCandidate, RaidScheduleCandidate, RaidSchedulerCheck,
};
use crate::foundation::ScopeOwnershipGeneration;
use oteryn_gameplay_encounter_rules::raid::{CatalogueSchedule, RaidCatalogue};

#[test]
fn w05d_halloween_hare_without_exact_waves_spawns_nothing() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async |admin| {
                let h = Harness::create_with_admission_seed(admin, "halloween_hare", true, false).await?;
                eprintln!("w05dcake-marker: w05d_halloween_hare_without_exact_waves_spawns_nothing db ready");
                let result = exercise(&h).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Halloween Hare thread panicked")??;
    Ok(())
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
    let catalogue = RaidCatalogue::embedded().map_err(debug)?;
    let key = crate::content::ProductionKey::new("oteryn:raid.kazordoon_raids.the_halloween_hare")
        .map_err(debug)?;
    let hare = catalogue.get(&key).ok_or("Hare calendar entry")?;
    assert_eq!(hare.variants.len(), 1, "the source wave is retained");
    assert_eq!(
        hare.inactive_reasons,
        ["calendar_firing_anchor_unbound:immediate_post_save"]
    );
    assert!(hare.select(bundle, id(81)).is_err());

    // Retain the original missing-wave case after the production record gained its wave.
    // Give it a complete recurring calendar rule so that the missing waves stand alone.
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
                "interval_ms": 86_400_000,
                "repeat": true
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
    let CatalogueSchedule::Calendar {
        window: Some(window),
        interval_ms,
        repeat,
    } = hare.schedule
    else {
        return Err("Hare must retain its annual calendar window".into());
    };
    assert_eq!((window.start_month, window.start_day), (10, 31));
    assert_eq!((window.end_month, window.end_day), (10, 31));
    assert_eq!(interval_ms, None);
    assert!(!repeat);

    // Apply the production scheduler's active-catalogue filter before proposing
    // a firing. Other raids are outside this case's catalogue selection.
    let firing_id = id(81);
    let candidates = catalogue
        .active()
        .chain(without_waves.active())
        .filter(|raid| raid.key == key.as_str())
        .map(|raid| -> TestResult<_> {
            let selected = raid.select(bundle, firing_id).map_err(debug)?;
            Ok(RaidScheduleCandidate {
                key: key.clone(),
                definition: selected.definition,
                eligible: true,
                calendar: None,
                firing_id,
                global_margin_ms: 0,
                lua: None,
                calendar_window: Some(CalendarRaidScheduleCandidate {
                    window: window.window(),
                    interval_ms: interval_ms.ok_or("Hare requires exact timing")?,
                    repeat,
                }),
            })
        })
        .collect::<TestResult<Vec<_>>>()?;

    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'hare-test-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,TIMESTAMPTZ '2026-10-31 10:00 Europe/Berlin','2026-10')")
        .bind(world.as_bytes().as_slice()).bind(bundle.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let now_ms = sqlx::query_scalar(
        "SELECT (extract(epoch FROM TIMESTAMPTZ '2026-10-31 16:00 Europe/Berlin') * 1000)::bigint",
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
        candidates,
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
    Ok(())
}
