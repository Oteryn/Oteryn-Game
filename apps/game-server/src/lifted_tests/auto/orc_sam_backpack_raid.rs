use crate::bestiary_postgres_harness::{CHANNEL, Harness, TestResult, WORLD, debug, id};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::raid_runner::CommittedRaidAction;
use crate::durability::raid_schedule::RaidSchedulerCheck;
use crate::foundation::{ChannelId, ChannelRuntimeV1, ScopeOwnershipGeneration, WorldId};
use crate::node::raid_scheduler::{RaidScheduler, candidates};
use oteryn_gameplay_encounter_rules::raid::{
    CHECK_INTERVAL_MS, RaidAction, RaidCatalogue, RaidPolicy, RaidSchedule,
};
use tokio::sync::Mutex;

const RAID: &str = "raid:venore.orcsambackpack";
const CREATURE: &str = "oteryn:creature.orc_sam_backpack";

#[test]
fn orc_sam_backpack_raid_postgres_scheduled_spawn_and_source_loot() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let run = || -> TestResult {
                crate::bestiary_postgres_harness::runtime()?.block_on(async {
                    let h = Harness::create(admin, "orc_sam_backpack_schedule", false).await?;
                    let result = scenario(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup
                })
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Orc Sam Backpack scheduler thread failed")?
        .map_err(Box::<dyn std::error::Error>::from)
}

async fn scenario(h: &Harness) -> TestResult {
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let bundle = [7; 32];
    let runtime = Mutex::new(
        ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            h.node.fact().node_id(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::test(world),
        )
        .map_err(debug)?,
    );
    let scheduler = RaidScheduler {
        root: &h.root,
        character: &authority,
        node: &h.node,
        runtime: &runtime,
        map: None,
        world,
        channel,
        generation: 1,
        reset_epoch: 1,
        bundle_digest: Some(bundle),
        active_layers_epoch: None,
        active_layers: Vec::new(),
    };
    let catalogue = RaidCatalogue::embedded().map_err(debug)?;
    let proposal = candidates(&catalogue, &scheduler, 0, bundle)
        .map_err(debug)?
        .into_iter()
        .find(|proposal| proposal.key.as_str() == RAID)
        .ok_or("Orc Sam Backpack must enter the active scheduler candidates")?;
    assert_eq!(proposal.global_margin_ms, 43_200_000);
    assert!(matches!(
        proposal.definition.policy,
        RaidPolicy::Random {
            mean_interval_s: 86_400,
            ..
        }
    ));

    // Derive the first winning ordinal with the production planner so the durable
    // check retains the source odds and needs only one due PostgreSQL transaction.
    let mut schedule =
        RaidSchedule::initial(1, i64::from(proposal.global_margin_ms)).map_err(debug)?;
    for _ in 0..20_000 {
        let transition = schedule
            .plan(
                world,
                &proposal.key,
                &proposal.definition,
                RaidAction::Check {
                    now_ms: schedule.next_check_at_ms,
                    eligible: true,
                    calendar: None,
                    firing_id: proposal.firing_id,
                    bundle_digest: bundle,
                },
            )
            .map_err(debug)?;
        if transition.after().latest_firing.is_some() {
            break;
        }
        schedule = transition.after().clone();
    }
    let now = schedule.next_check_at_ms;
    let due = candidates(&catalogue, &scheduler, now, bundle)
        .map_err(debug)?
        .into_iter()
        .find(|candidate| candidate.key == proposal.key)
        .ok_or("due Orc Sam Backpack candidate")?;
    let firing = due.firing_id;
    let definition = due.definition.clone();
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'orc-raid-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(world.as_bytes().as_slice()).bind(bundle.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO game_world_raid_scheduler_clocks VALUES(encode($1,'hex')::uuid,1,0)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO game_world_raid_schedules(world_id,raid_key,revision,reset_epoch,check_ordinal,next_firing_ordinal,next_check_at_ms,retired) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,1,$4::text::numeric,1,$5,false)")
        .bind(world.as_bytes().as_slice()).bind(RAID).bind(schedule.revision.to_string())
        .bind(schedule.check_ordinal.to_string()).bind(now).execute(&h.pool).await?;
    let request = RaidSchedulerCheck {
        world,
        channel,
        generation,
        reset_epoch: 1,
        now_ms: now,
        bundle_digest: bundle,
        candidates: vec![due],
    };
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings,
        vec![(firing, proposal.key)],
    );
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request)
            .await?
            .firings
            .is_empty()
    );
    assert!(
        h.root
            .advance_raid_run(
                &h.node,
                firing,
                channel,
                generation,
                definition.clone(),
                now + 999
            )
            .await?
            .is_none()
    );
    let step = h
        .root
        .advance_raid_run(
            &h.node,
            firing,
            channel,
            generation,
            definition,
            now + 1_000,
        )
        .await?
        .ok_or("scheduled Orc Sam Backpack wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = step.action else {
        return Err("scheduled Orc Sam Backpack must spawn a wave".into());
    };
    assert_eq!(creatures.len(), 1);
    assert_eq!(creatures[0].creature.as_str(), CREATURE);
    assert_eq!(
        creatures[0].area_anchor.as_str(),
        "raid-anchor:32964_31735_7-32981_31756_7"
    );
    assert!(now >= CHECK_INTERVAL_MS);

    let (controller, _) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native raid content")?;
    let facts = native
        .reward_table()
        .encounter_facts(CREATURE)
        .ok_or("raid Orc facts")?;
    assert_eq!(facts.display_name, "Orc");
    assert_eq!(facts.maximum_health, Some(70));
    assert_eq!(facts.experience, Some(25));
    let reward = native.reward_table().row(CREATURE).map_err(debug)?;
    assert!(reward.boss.is_none());
    assert_eq!(
        reward
            .corpse_item
            .as_ref()
            .ok_or("Orc corpse")?
            .production_key,
        "oteryn:item.tibia.i5966"
    );
    let source = [
        (3031, 848_100, 14),
        (3244, 1_000_000, 1),
        (3273, 58_500, 1),
        (3274, 49_600, 1),
        (3376, 29_500, 1),
        (3378, 78_600, 1),
        (3426, 73_000, 1),
        (3577, 101_600, 1),
        (10196, 2_100, 1),
        (11479, 5_900, 1),
        (23986, 10_000, 1),
    ];
    assert_eq!(reward.loot_table.entries.len(), source.len());
    for (entry, (id, chance, max)) in reward.loot_table.entries.iter().zip(source) {
        assert_eq!(
            entry.item.production_key,
            format!("oteryn:item.tibia.i{id}")
        );
        assert_eq!(
            (entry.min_count, entry.max_count, entry.probability_ppm),
            (1, max, Some(chance))
        );
    }
    Ok(())
}
