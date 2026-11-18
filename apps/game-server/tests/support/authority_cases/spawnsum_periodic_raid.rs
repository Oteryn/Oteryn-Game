//! The World scheduler admits one deterministic boss on each due interval.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::raid_runner::{CommittedRaidAction, RaidPlacementOutcome};
use crate::durability::raid_schedule::{RaidScheduleCandidate, RaidSchedulerCheck};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::RaidCatalogue;
use std::time::Duration;

#[test]
fn spawnsum_periodic_raid_scheduler_admits_one_boss_after_thirteen_hours() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let h =
                        fixture::Harness::create(admin, "spawnsum_periodic_raid", false).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "periodic raid scheduler case panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    let channel = ChannelId::decode(&fixture::id(fixture::CHANNEL)).map_err(fixture::debug)?;
    let digest = [7_u8; 32];
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(world.as_bytes().as_slice()).bind(digest.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let key = ProductionKey::new("raid:poi.rare_bosses").map_err(fixture::debug)?;
    let catalogue = RaidCatalogue::embedded().map_err(fixture::debug)?;
    let raid = catalogue
        .get(&key)
        .ok_or("missing periodic Pits of Inferno schedule")?;
    assert_eq!(raid.variants.len(), 7);
    let generation = ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?;
    let tick = |now_ms, firing_id, definition| {
        h.root.check_raid_schedules(
            &authority,
            &h.node,
            RaidSchedulerCheck {
                world,
                channel,
                generation,
                reset_epoch: 1,
                now_ms,
                bundle_digest: digest,
                candidates: vec![RaidScheduleCandidate {
                    key: key.clone(),
                    definition,
                    eligible: true,
                    calendar: None,
                    firing_id,
                    global_margin_ms: 0,
                    lua: None,
                    calendar_window: None,
                }],
            },
        )
    };
    let firing_id = fixture::id(88);
    let selected = raid.select(digest, firing_id).map_err(fixture::debug)?;
    let definition = selected.definition;
    assert!(
        tick(0, firing_id, definition.clone())
            .await?
            .firings
            .is_empty()
    );
    let early = advance_clock(Duration::from_millis(46_799_999)).await?;
    assert!(
        tick(early, firing_id, definition.clone())
            .await?
            .firings
            .is_empty()
    );
    let due = early + advance_clock(Duration::from_millis(1)).await?;
    assert_eq!(
        tick(due, firing_id, definition.clone()).await?.firings,
        [(firing_id, key.clone())]
    );
    let wave = h
        .root
        .advance_raid_run(
            &h.node,
            firing_id,
            channel,
            generation,
            definition.clone(),
            due,
        )
        .await?
        .ok_or("periodic boss wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = &wave.action else {
        return Err("periodic raid must start with its summon wave".into());
    };
    let [boss] = creatures.as_slice() else {
        return Err("periodic wave must summon exactly one boss".into());
    };
    assert_eq!(boss.creature, definition.waves[0].entries[0].creature);
    h.root
        .acknowledge_raid_effect(
            &h.node,
            wave.identity,
            vec![RaidPlacementOutcome {
                identity: boss.identity,
                placed: true,
            }],
            due,
        )
        .await?;
    let end = h
        .root
        .advance_raid_run(
            &h.node,
            firing_id,
            channel,
            generation,
            definition.clone(),
            due,
        )
        .await?
        .ok_or("periodic wave completion")?;
    assert!(matches!(end.action, CommittedRaidAction::Complete { .. }));
    h.root
        .acknowledge_raid_effect(&h.node, end.identity, vec![], due)
        .await?;

    assert!(tick(due, firing_id, definition).await?.firings.is_empty());
    let next_id = fixture::id(89);
    let next = raid.select(digest, next_id).map_err(fixture::debug)?;
    let second = due + advance_clock(Duration::from_secs(13 * 60 * 60)).await?;
    assert_eq!(
        tick(second, next_id, next.definition).await?.firings,
        [(next_id, key.clone())]
    );
    Ok(())
}

async fn advance_clock(duration: Duration) -> fixture::TestResult<i64> {
    tokio::time::pause();
    let start = tokio::time::Instant::now();
    tokio::time::advance(duration).await;
    let elapsed = start.elapsed();
    tokio::time::resume();
    Ok(i64::try_from(elapsed.as_millis())?)
}
