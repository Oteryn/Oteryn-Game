//! The node's catalogue pass must wait for the committed October 31 save.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::raid_runner::{CommittedRaidAction, RaidPlacementOutcome};
use crate::durability::raid_schedule::RaidWrite;
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
use oteryn_gameplay_encounter_rules::raid::RaidCatalogue;
use production_server::node::{RaidScheduleTick, check_raid_tick};

#[test]
fn w05d_hare_node_tick_fires_once_after_committed_october_31_save() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let h = fixture::Harness::create(admin, "w05d_hare_node_tick", false).await?;
                    let result = exercise(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "Hare scheduler thread panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    let channel = ChannelId::decode(&fixture::id(fixture::CHANNEL)).map_err(fixture::debug)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?;
    let bundle_digest = [7_u8; 32];
    let key = "oteryn:raid.kazordoon_raids.the_halloween_hare";
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../content/encounters/raids/catalogue.json"
    ))?;
    let hare = catalogue["records"]
        .as_array()
        .ok_or("catalogue records")?
        .iter()
        .find(|raid| raid["key"] == key)
        .ok_or("retained Hare record")?;
    let catalogue = RaidCatalogue::from_json(
        &serde_json::json!({
            "schema": catalogue["schema"],
            "records": [hare]
        })
        .to_string(),
    )?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let save_ms: i64 = sqlx::query_scalar(
        "SELECT (extract(epoch FROM TIMESTAMPTZ '2026-10-31 06:00 Europe/Berlin') * 1000)::bigint",
    )
    .fetch_one(&h.pool)
    .await?;
    let committed_ms = save_ms + 5_000;
    sqlx::query(
        "INSERT INTO game_world_reset_epochs
         (world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month)
         VALUES (encode($1,'hex')::uuid,1,$2,'hare-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,TIMESTAMPTZ '2026-10-30 06:00 Europe/Berlin','2026-10'),
                (encode($1,'hex')::uuid,2,$2,'hare-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($3::double precision/1000),'2026-10')",
    ).bind(world.as_bytes().as_slice()).bind(bundle_digest.as_slice()).bind(committed_ms)
        .execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;

    let mut tick = RaidScheduleTick {
        world,
        channel,
        generation,
        reset_epoch: 1,
        now_ms: save_ms - 1,
        bundle_digest,
    };
    assert!(
        check_raid_tick(&h.root, &authority, &h.node, &catalogue, &tick, None)
            .await??
            .firings
            .is_empty()
    );
    tick.now_ms = save_ms;
    assert!(
        check_raid_tick(&h.root, &authority, &h.node, &catalogue, &tick, None)
            .await??
            .firings
            .is_empty(),
        "wall clock alone does not open an occurrence"
    );
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;

    tick.reset_epoch = 2;
    assert!(
        check_raid_tick(&h.root, &authority, &h.node, &catalogue, &tick, None)
            .await??
            .firings
            .is_empty(),
        "a clock sample before activation cannot fire"
    );
    tick.now_ms = committed_ms;
    let outcome = check_raid_tick(&h.root, &authority, &h.node, &catalogue, &tick, None).await??;
    let [(firing, fired_key)] = outcome.firings.as_slice() else {
        return Err("the first post-commit tick must admit exactly one Hare firing".into());
    };
    assert_eq!(fired_key.as_str(), key);
    let selected = catalogue
        .get(fired_key)
        .ok_or("Hare")?
        .select(bundle_digest, *firing)?;
    let announcement = h
        .root
        .advance_raid_run(
            &h.node,
            *firing,
            channel,
            generation,
            selected.definition.clone(),
            committed_ms,
        )
        .await?
        .ok_or("immediate Hare announcement")?;
    assert_eq!(
        announcement.action,
        CommittedRaidAction::Announcement {
            text: selected.definition.announcements[0].text.clone(),
            advance: true,
        }
    );

    assert_eq!(
        h.root
            .acknowledge_raid_effect(&h.node, announcement.identity, vec![], committed_ms)
            .await?,
        RaidWrite::Applied,
    );
    let wave = h
        .root
        .advance_raid_run(
            &h.node,
            *firing,
            channel,
            generation,
            selected.definition.clone(),
            committed_ms,
        )
        .await?
        .ok_or("immediate Hare wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = &wave.action else {
        return Err("expected Hare wave after announcement".into());
    };
    let [hare] = creatures.as_slice() else {
        return Err("expected one creature request".into());
    };
    assert_eq!(
        hare.creature,
        selected.definition.waves[0].entries[0].creature
    );
    // This scheduler case models the runner's placement acknowledgement. The
    // dispatch case exercises the real map and actor placement separately.
    assert_eq!(
        h.root
            .acknowledge_raid_effect(
                &h.node,
                wave.identity,
                vec![RaidPlacementOutcome {
                    identity: hare.identity,
                    placed: true
                }],
                committed_ms
            )
            .await?,
        RaidWrite::Applied,
    );
    let end = h
        .root
        .advance_raid_run(
            &h.node,
            *firing,
            channel,
            generation,
            selected.definition,
            committed_ms,
        )
        .await?
        .ok_or("completed Hare wave")?;
    assert_eq!(end.action, CommittedRaidAction::Complete { cleanup: false });
    assert_eq!(
        h.root
            .acknowledge_raid_effect(&h.node, end.identity, vec![], committed_ms)
            .await?,
        RaidWrite::Applied,
    );

    for ms in [committed_ms, committed_ms + 60_000] {
        tick.now_ms = ms;
        assert!(
            check_raid_tick(&h.root, &authority, &h.node, &catalogue, &tick, None)
                .await??
                .firings
                .is_empty()
        );
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_world_raid_firings WHERE raid_key=$1")
            .bind(key)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(count, 1);
    eprintln!("POSTGRES_CASE_COMPLETED W05d node tick confirmed reset, announcement and replay");
    Ok(())
}
