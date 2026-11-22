use crate::bestiary_postgres_harness::{CHANNEL, Harness, TestResult, WORLD, debug, id, runtime};
use crate::durability::raid_runner::{CommittedRaidAction, RaidPlacementOutcome};
use crate::durability::raid_schedule::{RaidScheduleCandidate, RaidSchedulerCheck, RaidWrite};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::{
    Announcement, AnnouncementKind, RaidDefinition, RaidPolicy, Wave, WaveEntry,
};

#[test]
fn gaz_haragoth_prison_raid_requires_a_durable_first_defeat() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let run = || -> TestResult {
                runtime()?.block_on(async {
                    let h = Harness::create(admin, "gaz_first_defeat", false).await?;
                    let result = exercise(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup
                })
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Gaz'haragoth case thread")?
        .map_err(Box::<dyn std::error::Error>::from)?;
    eprintln!("POSTGRES_CASE_COMPLETED gaz_haragoth_prison_raid_requires_a_durable_first_defeat");
    Ok(())
}

async fn exercise(h: &Harness) -> TestResult {
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let digest = [7_u8; 32];
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'raid-test-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(world.as_bytes().as_slice()).bind(digest.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let prison = ProductionKey::new("raid:roshamuul.gaz_haragoth").map_err(debug)?;
    let hill = ProductionKey::new("raid:roshamuul.gaz_haragoth_first").map_err(debug)?;
    // Crystal 201189f data-global/raids/roshamuul/gaz_haragoth.xml.
    let definition = RaidDefinition {
        policy: RaidPolicy::Random {
            mean_interval_s: 60,
            min_gap_s: 0,
        },
        despawn_after_ms: None,
        announcements: [
            (
                1_000,
                "Something out of the ordinary, it's better we hide now!",
            ),
            (
                20_000,
                "His strength is unlimited, the demons kneeling before him .!",
            ),
            (
                50_000,
                "The dreaded Gaz'Haragoth is invading the land from prison.",
            ),
            (80_000, "Leave now mortals, beware Gaz'Haragoth."),
        ]
        .into_iter()
        .map(|(delay_ms, text)| Announcement {
            delay_ms,
            text: text.into(),
            kind: AnnouncementKind::Warning,
        })
        .collect(),
        waves: vec![Wave {
            delay_ms: 80_000,
            entries: vec![WaveEntry {
                creature: ProductionKey::new("oteryn:creature.gaz_haragoth").map_err(debug)?,
                count: 1,
                area_anchor: ProductionKey::new("raid:roshamuul.gaz_haragoth").map_err(debug)?,
            }],
        }],
    };
    let candidate = |key, tag| RaidScheduleCandidate {
        key,
        definition: definition.clone(),
        eligible: true,
        calendar: None,
        firing_id: id(tag),
        global_margin_ms: 0,
        lua: None,
        calendar_window: None,
    };
    let mut request = RaidSchedulerCheck {
        world,
        channel,
        generation,
        reset_epoch: 1,
        now_ms: 0,
        bundle_digest: digest,
        candidates: vec![candidate(prison.clone(), 80), candidate(hill.clone(), 81)],
    };
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty()
    );
    request.now_ms = 60_000;
    let started = h
        .root
        .check_raid_schedules(&authority, &h.node, request.clone())
        .await?;
    assert_eq!(started.firings, vec![(id(81), hill)]);
    request.now_ms += 80_000;
    for _ in 0..definition.announcements.len() {
        let step = h
            .root
            .advance_raid_run(
                &h.node,
                id(81),
                channel,
                generation,
                definition.clone(),
                request.now_ms,
            )
            .await?
            .ok_or("announcement")?;
        assert!(matches!(
            step.action,
            CommittedRaidAction::Announcement { .. }
        ));
        assert_eq!(
            h.root
                .acknowledge_raid_effect(&h.node, step.identity, vec![], request.now_ms)
                .await?,
            RaidWrite::Applied
        );
    }
    let step = h
        .root
        .advance_raid_run(
            &h.node,
            id(81),
            channel,
            generation,
            definition.clone(),
            request.now_ms,
        )
        .await?
        .ok_or("wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = step.action else {
        return Err("Gaz wave".into());
    };
    let boss = creatures.first().ok_or("Gaz boss")?.identity;
    assert_eq!(
        h.root
            .record_raid_creature_deaths(&h.node, step.identity, vec![boss])
            .await?,
        RaidWrite::CompareFailed
    );
    assert_eq!(
        h.root
            .acknowledge_raid_effect(
                &h.node,
                step.identity,
                vec![RaidPlacementOutcome {
                    identity: boss,
                    placed: true
                }],
                request.now_ms
            )
            .await?,
        RaidWrite::Applied
    );
    assert_eq!(
        h.root
            .record_raid_creature_deaths(&h.node, step.identity, vec![boss])
            .await?,
        RaidWrite::Applied
    );
    assert_eq!(
        h.root
            .record_raid_creature_deaths(&h.node, step.identity, vec![boss])
            .await?,
        RaidWrite::Replayed
    );
    request.candidates = vec![candidate(prison.clone(), 80)];
    h.root
        .check_raid_schedules(&authority, &h.node, request.clone())
        .await?;
    request.now_ms += 60_000;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, request)
            .await?
            .firings,
        vec![(id(80), prison)]
    );
    Ok(())
}
