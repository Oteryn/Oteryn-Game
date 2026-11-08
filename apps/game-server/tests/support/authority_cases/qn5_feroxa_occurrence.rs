//! Crystal 9f5a72c grimvale.lua checks stored Chance before drawing its replacement.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::raid_schedule::{RaidScheduleCandidate, RaidSchedulerCheck};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::RaidCatalogue;

#[test]
fn qn5_feroxa_unset_chance_selects_first_due_full_moon_check() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let start = std::time::Instant::now();
                    for initial_chance in [None, Some(10)] {
                        let h =
                            fixture::Harness::create(admin.clone(), "qn5_feroxa_occurrence", false)
                                .await?;
                        let result = exercise(&h, initial_chance).await;
                        h.cleanup().await?;
                        result?;
                    }
                    assert!(start.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "QN5_M52_OCCURRENCE_PG_EXECUTED elapsed={:?}",
                        start.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "Feroxa occurrence case panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness, initial_chance: Option<i16>) -> fixture::TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QN5_M52_OCCURRENCE_POSTGRES_READY version={version}");
    let world = fixture::scope()?.world_id();
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,make_timestamptz(2031,6,13,12,0,0,'UTC'),'2031-06')")
        .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let key = ProductionKey::new("raid:edron.feroxa").map_err(fixture::debug)?;
    let source = RaidCatalogue::embedded()
        .map_err(fixture::debug)?
        .get(&key)
        .ok_or("native Feroxa")?
        .select([7; 32], fixture::id(88))
        .map_err(fixture::debug)?;
    let mut request = RaidSchedulerCheck {
        world,
        channel: ChannelId::decode(&fixture::id(fixture::CHANNEL)).map_err(fixture::debug)?,
        generation: ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?,
        reset_epoch: 1,
        now_ms: 0,
        bundle_digest: [7; 32],
        candidates: vec![RaidScheduleCandidate {
            key,
            definition: source.definition,
            eligible: true,
            calendar: None,
            firing_id: fixture::id(88),
            global_margin_ms: 0,
            lua: None,
            calendar_window: None,
        }],
    };
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty()
    );
    if let Some(chance) = initial_chance {
        sqlx::query("INSERT INTO game_world_feroxa_occurrences(world_id,reset_epoch,chance) VALUES(encode($1,'hex')::uuid,1,$2)")
            .bind(world.as_bytes().as_slice()).bind(chance).execute(&h.pool).await?;
    }
    let mut prior_chance = initial_chance.unwrap_or(-1);
    let mut selected = false;
    for minute in 1..=100 {
        request.now_ms = minute * 60_000;
        let chosen = h
            .root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?;
        assert_eq!(
            chosen.firings.len(),
            usize::from(prior_chance <= 5),
            "selection tests the preceding stored chance, including unset -1, before drawing its replacement"
        );
        let state: (i16, i16, Option<i64>) = sqlx::query_as("SELECT chance,phase,selected_at_ms FROM game_world_feroxa_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
            .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert!(
            h.root
                .check_raid_schedules(&authority, &h.node, request.clone())
                .await?
                .firings
                .is_empty(),
            "same due check cannot select or draw twice"
        );
        let replay: (i16, i16, Option<i64>) = sqlx::query_as("SELECT chance,phase,selected_at_ms FROM game_world_feroxa_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
            .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            state, replay,
            "a replay retains the exact persisted chance and Active state"
        );
        if prior_chance <= 5 {
            assert_eq!(chosen.firings[0].1.as_str(), "raid:edron.feroxa");
            assert_eq!(state, (prior_chance, 1, Some(request.now_ms)));
            selected = true;
            break;
        }
        assert!(
            (1..=10).contains(&state.0),
            "source replacement draw is inclusive 1..10"
        );
        assert_eq!((state.1, state.2), (0, None));
        prior_chance = state.0;
    }
    assert!(
        selected,
        "the fixed deterministic sequence reaches a qualifying prior chance"
    );
    let selected_at = request.now_ms;
    // Source Active remains set after the boss is gone, until the next reset.
    sqlx::query(
        "UPDATE game_world_raid_firings SET state=2 WHERE firing_id=encode($1,'hex')::uuid",
    )
    .bind(fixture::id(88).as_slice())
    .execute(&h.pool)
    .await?;
    request.now_ms += 60_000;
    request.candidates[0].firing_id = fixture::id(89);
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty(),
        "a completed Feroxa does not clear source Active in the same occurrence"
    );
    // A pre-migration firing has no Feroxa state row. Its admitted identity
    // still establishes Active, even if the boss completed before the upgrade.
    sqlx::query("DELETE FROM game_world_feroxa_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    request.now_ms += 60_000;
    request.candidates[0].firing_id = fixture::id(90);
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request)
            .await?
            .firings
            .is_empty(),
        "upgrading an existing occurrence cannot spawn a second Feroxa"
    );
    let upgraded: (i16, Option<i64>) = sqlx::query_as("SELECT phase,selected_at_ms FROM game_world_feroxa_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(upgraded, (1, Some(selected_at)));
    Ok(())
}
