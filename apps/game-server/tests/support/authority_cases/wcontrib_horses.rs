//! WCONTRIB: Horse actions drive the real native raid scheduler and runner.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::DurabilityError;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::npc_travel::{NpcTravelGate, TravelRefusal};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::raid_runner::CommittedRaidAction;
use crate::durability::raid_schedule::{RaidScheduleCandidate, RaidSchedulerCheck};
use crate::durability::world_event::changes::actions;
use crate::foundation::{
    ChannelId, CommandId, CommandRef, GameSessionId, ScopeOwnershipGeneration,
};
use oteryn_gameplay_content::content::npc_catalogue::travel::TravelRoute;
use oteryn_gameplay_content::content::{
    ProductionKey, ProjectV2TravelDestination, ProjectV2TravelRoute,
};
use oteryn_gameplay_encounter_rules::raid::{RaidCatalogue, RaidDefinition};
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestStateCatalogue};
use std::sync::Arc;

#[test]
fn wcontrib_horse_pg_escape_zero_raid_expiry_native_waves_and_travel() -> fixture::TestResult {
    run_thread(false)
}
#[test]
fn wcontrib_horse_pg_first_escape_three_hour_boundary() -> fixture::TestResult {
    run_thread(true)
}
fn run_thread(timer: bool) -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name("wcontrib-horse-pg".into())
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(exercise(admin, timer))
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "horse PG thread failed")??;
    Ok(())
}
async fn exercise(admin: String, timer: bool) -> fixture::TestResult {
    let started = std::time::Instant::now();
    let h = fixture::Harness::create(
        admin,
        if timer {
            "wcontrib_horse_timer"
        } else {
            "wcontrib_horse_native"
        },
        true,
    )
    .await?;
    let result = if timer {
        timer_boundary(&h).await
    } else {
        native_sequence(&h).await
    };
    h.cleanup().await?;
    result?;
    eprintln!(
        "WCONTRIB_HORSE_PG_EXECUTED: {} ; elapsed={:.3}s",
        if timer {
            "actual escape and three-hour activation boundary"
        } else {
            "actual Quest escapes, renamed replay refusal, native raid waves 9/8/24/9, LOST cleanup and paid gate"
        },
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
async fn now(h: &fixture::Harness) -> fixture::TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&h.pool)
            .await?,
    )
}
async fn seed(h: &fixture::Harness) -> fixture::TestResult<Arc<QuestStateCatalogue>> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(fixture::id(fixture::WORLD).as_slice()).bind([1_u8;32].as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(fixture::id(fixture::WORLD).as_slice())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    h.root
        .world_changes(fixture::scope()?.world_id(), 1)
        .await
        .map_err(|error| format!("Horse initial world snapshot: {error:?}"))?
        .ok_or("horse initial world")?;
    Ok(Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(fixture::debug)?
            .catalogue()
            .clone(),
    ))
}
async fn revision(h: &fixture::Harness) -> fixture::TestResult<u64> {
    let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    Ok(revision.parse()?)
}
async fn action(
    h: &fixture::Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    catalogue: &Arc<QuestStateCatalogue>,
    key: &str,
    ordinal: u64,
) -> fixture::TestResult<QuestTransitionOutcome> {
    let command = CommandRef::new(
        GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?,
        CommandId::new(ordinal).map_err(fixture::debug)?,
    );
    Ok(h.root
        .commit_character_quest_transition(
            authority,
            &h.node,
            fixture::fence(revision(h).await?)?,
            QuestTransitionRequest {
                transition_key: key.into(),
                cause: QuestCause::Command(command),
            },
            Arc::clone(catalogue),
        )
        .await
        .map_err(|error| format!("Horse Quest action {key}/{ordinal}: {error:?}"))?)
}
fn definition(key: &str) -> fixture::TestResult<RaidDefinition> {
    Ok(RaidCatalogue::embedded()
        .map_err(fixture::debug)?
        .get(&ProductionKey::new(key).map_err(fixture::debug)?)
        .ok_or("native Horse projection")?
        .select([7_u8; 32], fixture::id(80))
        .map_err(fixture::debug)?
        .definition)
}
async fn check(
    h: &fixture::Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    key: &str,
    seed: u8,
) -> fixture::TestResult<Vec<([u8; 16], ProductionKey)>> {
    Ok(h.root
        .check_raid_schedules(
            authority,
            &h.node,
            RaidSchedulerCheck {
                world: fixture::scope()?.world_id(),
                channel: ChannelId::decode(&fixture::id(fixture::CHANNEL))
                    .map_err(fixture::debug)?,
                generation: ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?,
                reset_epoch: 1,
                now_ms: now(h).await?,
                bundle_digest: [1_u8; 32],
                candidates: vec![RaidScheduleCandidate {
                    key: ProductionKey::new(key).map_err(fixture::debug)?,
                    definition: definition(key)?,
                    eligible: true,
                    calendar: None,
                    firing_id: fixture::id(seed),
                    global_margin_ms: 0,
                    lua: None,
                    calendar_window: None,
                }],
            },
        )
        .await
        .map_err(|error| format!("Horse scheduler {key}/{seed}: {error:?}"))?
        .firings)
}
async fn wave(h: &fixture::Harness, key: &str, seed: u8, count: usize) -> fixture::TestResult {
    let fired: i64 = sqlx::query_scalar(
        "SELECT fired_at_ms FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid",
    )
    .bind(fixture::id(seed).as_slice())
    .fetch_one(&h.pool)
    .await?;
    // Wait for the actual source wave deadline, not a guessed fixture delay.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    while now(h).await? < fired + 1000 {
        if tokio::time::Instant::now() >= deadline {
            return Err("native Horse wave did not become due".into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let step = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(seed),
            ChannelId::decode(&fixture::id(fixture::CHANNEL)).map_err(fixture::debug)?,
            ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?,
            definition(key)?,
            now(h).await?,
        )
        .await
        .map_err(|error| format!("Horse native wave {key}/{seed}: {error:?}"))?
        .ok_or("native Horse wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = step.action else {
        return Err("Horse projection did not use native wave owner".into());
    };
    assert_eq!(creatures.len(), count);
    let mut species = std::collections::BTreeMap::new();
    for creature in &creatures {
        *species.entry(creature.creature.as_str()).or_insert(0_usize) += 1;
    }
    let expected = if key.ends_with("wild") {
        std::collections::BTreeMap::from([
            ("oteryn:creature.wild_horse", 6),
            ("oteryn:creature.horse", 6),
            ("oteryn:creature.grey_horse", 6),
            ("oteryn:creature.brown_horse", 6),
        ])
    } else {
        std::collections::BTreeMap::from([
            ("oteryn:creature.horse", 4),
            ("oteryn:creature.grey_horse", 2),
            ("oteryn:creature.brown_horse", count - 6),
        ])
    };
    assert_eq!(
        species, expected,
        "native sourced horse species and count are preserved"
    );
    let persisted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_world_raid_creatures WHERE firing_id=encode($1,'hex')::uuid",
    )
    .bind(fixture::id(seed).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(usize::try_from(persisted)?, count);
    // No fabricated placed=true ACK: these are real durable native wave plans.
    // The existing LOST path below must invalidate their pending effects first.
    Ok(())
}
async fn lose(h: &fixture::Harness, seed: u8) -> fixture::TestResult {
    let step = h
        .root
        .claim_lost_raid_run(
            &h.node,
            fixture::id(seed),
            ChannelId::decode(&fixture::id(fixture::CHANNEL)).map_err(fixture::debug)?,
            ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?,
            now(h).await?,
        )
        .await
        .map_err(|error| format!("Horse LOST claim {seed}: {error:?}"))?
        .ok_or("obsolete Horse projection must become LOST")?;
    assert!(matches!(step.action, CommittedRaidAction::Lost));
    h.root
        .acknowledge_raid_effect(&h.node, step.identity, vec![], now(h).await?)
        .await
        .map_err(|error| format!("Horse LOST ACK {seed}: {error:?}"))?;
    Ok(())
}
async fn age_timer(
    h: &fixture::Harness,
    first_escape: i64,
    raid_until: Option<i64>,
) -> fixture::TestResult {
    // Compress elapsed real-world hours into historical timer DATA only. The
    // real current scheduler must still perform the transition and native wave.
    let mut live = h
        .root
        .live_world_changes(fixture::scope()?.world_id(), 1)
        .await
        .map_err(|error| format!("Horse historical Horse timer: {error:?}"))?
        .ok_or("historical Horse timer")?;
    live.horse.escape_started_ms = Some(first_escape);
    live.horse.raid_until_ms = raid_until;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_world_change_live SET state=$2 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(fixture::id(fixture::WORLD).as_slice()).bind(serde_json::to_value(live)?).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn native_sequence(h: &fixture::Harness) -> fixture::TestResult {
    let catalogue = seed(h).await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let world = fixture::scope()?.world_id();
    assert_eq!(
        h.root
            .live_world_changes(world, 1)
            .await
            .map_err(|error| format!("Horse stable: {error:?}"))?
            .ok_or("stable")?
            .horse
            .stable_horses,
        9
    );
    assert_eq!(
        h.root
            .quote_npc_travel(
                &authority,
                &h.node,
                fixture::fence(1)?,
                travel(),
                &AllowTravel
            )
            .await
            .map_err(|error| format!("Horse paid quote: {error:?}"))?,
        Ok(125)
    );
    assert_eq!(
        check(h, &authority, "raid:horsestation.stable.9", 80)
            .await?
            .len(),
        1
    );
    wave(h, "raid:horsestation.stable.9", 80, 9).await?;
    assert!(matches!(
        action(h, &authority, &catalogue, actions::HORSE_ESCAPE, 1).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert!(matches!(
        action(h, &authority, &catalogue, actions::HORSE_ESCAPE, 1).await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(
        h.root
            .live_world_changes(world, 1)
            .await
            .map_err(|error| format!("Horse first escape: {error:?}"))?
            .ok_or("first escape")?
            .horse
            .stable_horses,
        8
    );
    let before = revision(h).await?;
    assert_eq!(
        action(h, &authority, &catalogue, actions::HORSE_RETURN, 1).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert_eq!(revision(h).await?, before);
    assert_eq!(
        h.root
            .live_world_changes(world, 1)
            .await
            .map_err(|error| format!("Horse renamed escape: {error:?}"))?
            .ok_or("renamed escape")?
            .horse
            .stable_horses,
        8
    );
    assert!(
        check(h, &authority, "raid:horsestation.stable.8", 81)
            .await?
            .is_empty(),
        "old pending native projection must be cleaned before replacement"
    );
    lose(h, 80).await?;
    assert_eq!(
        check(h, &authority, "raid:horsestation.stable.8", 81)
            .await?
            .len(),
        1
    );
    wave(h, "raid:horsestation.stable.8", 81, 8).await?;
    for ordinal in 2..=9 {
        assert!(matches!(
            action(h, &authority, &catalogue, actions::HORSE_ESCAPE, ordinal).await?,
            QuestTransitionOutcome::Committed(_)
        ));
    }
    let escaped = h
        .root
        .live_world_changes(world, 1)
        .await
        .map_err(|error| format!("Horse zero horses: {error:?}"))?
        .ok_or("zero horses")?;
    assert_eq!(escaped.horse.stable_horses, 0);
    assert!(escaped.horse.raid_active(now(h).await?));
    assert_eq!(
        h.root
            .quote_npc_travel(
                &authority,
                &h.node,
                fixture::fence(revision(h).await?)?,
                travel(),
                &AllowTravel
            )
            .await
            .map_err(fixture::debug)?,
        Err(TravelRefusal::UnavailableRoute)
    );
    assert!(
        check(h, &authority, "raid:horsestation.wild", 82)
            .await?
            .is_empty()
    );
    lose(h, 81).await?;
    assert_eq!(
        check(h, &authority, "raid:horsestation.wild", 82)
            .await?
            .len(),
        1
    );
    wave(h, "raid:horsestation.wild", 82, 24).await?;
    let current = now(h).await?;
    age_timer(h, current - 10_800_001, Some(current - 1)).await?;
    assert!(
        check(h, &authority, "raid:horsestation.wild", 83)
            .await?
            .is_empty()
    );
    let restored = h
        .root
        .live_world_changes(world, 1)
        .await
        .map_err(|error| format!("Horse expired raid: {error:?}"))?
        .ok_or("expired raid")?;
    assert_eq!(restored.horse.stable_horses, 9);
    assert!(restored.horse.raid_until_ms.is_none());
    lose(h, 82).await?;
    // Three elapsed hours also make the existing stable source check due again.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_world_raid_schedules SET next_check_at_ms=$2 WHERE world_id=encode($1,'hex')::uuid AND raid_key='raid:horsestation.stable.9'")
        .bind(fixture::id(fixture::WORLD).as_slice()).bind(now(h).await?-1).execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(
        check(h, &authority, "raid:horsestation.stable.9", 84)
            .await?
            .len(),
        1
    );
    wave(h, "raid:horsestation.stable.9", 84, 9).await?;
    assert_eq!(
        h.root
            .quote_npc_travel(
                &authority,
                &h.node,
                fixture::fence(revision(h).await?)?,
                travel(),
                &AllowTravel
            )
            .await
            .map_err(|error| format!("Horse paid quote: {error:?}"))?,
        Ok(125)
    );
    drop(authority);
    drop(seal);
    Ok(())
}
async fn timer_boundary(h: &fixture::Harness) -> fixture::TestResult {
    let catalogue = seed(h).await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(
        action(h, &authority, &catalogue, actions::HORSE_ESCAPE, 1).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    let world = fixture::scope()?.world_id();
    let live = h
        .root
        .live_world_changes(world, 1)
        .await
        .map_err(|error| format!("Horse first timed escape: {error:?}"))?
        .ok_or("first timed escape")?;
    assert_eq!(live.horse.stable_horses, 8);
    assert!(live.horse.raid_until_ms.is_none());
    let mut tx = h.pool.begin().await?;
    let started = live.horse.escape_started_ms.ok_or("first escape time")?;
    let before = actions::horse_state(&mut tx, world, 1, started + 10_800_000 - 1)
        .await
        .map_err(|error| format!("Horse pre three hours: {error:?}"))?
        .ok_or("pre three hours")?;
    assert_eq!(before.stable_horses, 8);
    assert!(!before.raid_active(started + 10_800_000 - 1));
    tx.rollback().await?;
    age_timer(h, now(h).await? - 10_800_001, None).await?;
    assert_eq!(
        check(h, &authority, "raid:horsestation.wild", 90)
            .await?
            .len(),
        1
    );
    wave(h, "raid:horsestation.wild", 90, 24).await?;
    assert!(
        h.root
            .live_world_changes(world, 1)
            .await
            .map_err(|error| format!("Horse three-hour raid: {error:?}"))?
            .ok_or("three-hour raid")?
            .horse
            .raid_active(now(h).await?)
    );
    drop(authority);
    drop(seal);
    Ok(())
}
struct AllowTravel;
impl NpcTravelGate for AllowTravel {
    async fn travel_permitted(
        &self,
        _tx: &mut Type2Transaction<'_>,
        _account: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        Ok(true)
    }
}
fn travel() -> TravelRoute {
    TravelRoute {
        service: "oteryn:service.travel.palomino".into(),
        route: ProjectV2TravelRoute {
            key: "venore".into(),
            destination: ProjectV2TravelDestination {
                coordinate_frame: "global-target-2026-09-27".into(),
                x: 32850,
                y: 32124,
                floor: 7,
            },
            price: 125,
            premium: false,
            min_level: None,
        },
        keyword: "transport".into(),
        postman: false,
        carpet: false,
        held: false,
        yalahar_gate: false,
        yalahar_town_gate: None,
        quest_gate: None,
    }
}

#[test]
fn wcontrib_horse_pg_late_channel_reuses_done_stable_firing() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name("wcontrib-horse-late-pg".into())
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(late_channel(admin))
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "late Horse Channel thread failed")??;
    Ok(())
}
async fn late_channel(admin: String) -> fixture::TestResult {
    let started = std::time::Instant::now();
    let h = fixture::Harness::create(admin, "wcontrib_horse_late_channel", true).await?;
    let result = late_channel_case(&h).await;
    h.cleanup().await?;
    result?;
    eprintln!(
        "WCONTRIB_HORSE_LATE_CHANNEL_PG_EXECUTED: genuine assignment/readiness, same DONE firing attached, native Wave9, no original-channel duplication; elapsed={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
async fn late_channel_case(h: &fixture::Harness) -> fixture::TestResult {
    use crate::durability::raid_runner::RaidPlacementOutcome;
    use crate::durability::runtime_scope_assignment::{
        AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
    };
    use crate::foundation::RuntimeScopeRefV1;
    seed(h).await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let key = "raid:horsestation.stable.9";
    assert_eq!(check(h, &authority, key, 95).await?.len(), 1);
    wave(h, key, 95, 9).await?;
    let first_channel =
        ChannelId::decode(&fixture::id(fixture::CHANNEL)).map_err(fixture::debug)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?;
    let step = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(95),
            first_channel,
            generation,
            definition(key)?,
            now(h).await?,
        )
        .await
        .map_err(|error| format!("Horse late-channel initial wave replay: {error:?}"))?
        .ok_or("pending native stable wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = step.action else {
        return Err("stable wave replay".into());
    };
    // Explicit placement-refusal outcomes are truthful; no fake physical actors.
    h.root
        .acknowledge_raid_effect(
            &h.node,
            step.identity,
            creatures
                .into_iter()
                .map(|c| RaidPlacementOutcome {
                    identity: c.identity,
                    placed: false,
                })
                .collect(),
            now(h).await?,
        )
        .await?;
    let complete = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(95),
            first_channel,
            generation,
            definition(key)?,
            now(h).await?,
        )
        .await
        .map_err(|error| format!("Horse stable Complete: {error:?}"))?
        .ok_or("stable DONE action")?;
    assert!(matches!(
        complete.action,
        CommittedRaidAction::Complete { cleanup: false }
    ));
    h.root
        .acknowledge_raid_effect(&h.node, complete.identity, vec![], now(h).await?)
        .await
        .map_err(|error| format!("Horse stable Complete ACK: {error:?}"))?;
    let late_channel = ChannelId::decode(&fixture::id(44)).map_err(fixture::debug)?;
    let scope = RuntimeScopeRefV1::channel(fixture::scope()?.world_id(), late_channel);
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) SELECT session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,operation FROM unnest(ARRAY[1,2,3]::smallint[]) AS operation")
        .bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(44).as_slice()).execute(&h.pool).await?;
    let assignment = h
        .writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([44_u8; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(fixture::debug)?,
            command: AssignmentCommand::Assign {
                scope,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(assignment, AssignmentOutcome::Committed(_)));
    fixture::publish_readiness_for_scope(&h.pool, &h.root, &h.node, 1, scope).await?;
    h.root
        .check_raid_schedules(
            &authority,
            &h.node,
            RaidSchedulerCheck {
                world: fixture::scope()?.world_id(),
                channel: late_channel,
                generation,
                reset_epoch: 1,
                now_ms: now(h).await?,
                bundle_digest: [1_u8; 32],
                candidates: vec![RaidScheduleCandidate {
                    key: ProductionKey::new(key).map_err(fixture::debug)?,
                    definition: definition(key)?,
                    eligible: true,
                    calendar: None,
                    firing_id: fixture::id(96),
                    global_margin_ms: 0,
                    lua: None,
                    calendar_window: None,
                }],
            },
        )
        .await?;
    let firings:i64=sqlx::query_scalar("SELECT count(*) FROM game_world_raid_firings WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
        .bind(fixture::id(fixture::WORLD).as_slice()).bind(key).fetch_one(&h.pool).await?;
    assert_eq!(
        firings, 1,
        "late admission attaches to the same firing without another ordinal"
    );
    assert!(
        h.root
            .due_raid_firings(late_channel)
            .await?
            .iter()
            .any(|(id, _)| *id == fixture::id(95))
    );
    let late = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(95),
            late_channel,
            generation,
            definition(key)?,
            now(h).await?,
        )
        .await
        .map_err(|error| format!("Horse late-channel native wave: {error:?}"))?
        .ok_or("late Channel native stable wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = late.action else {
        return Err("late stable wave".into());
    };
    assert_eq!(creatures.len(), 9);
    assert!(
        h.root
            .advance_raid_run(
                &h.node,
                fixture::id(95),
                first_channel,
                generation,
                definition(key)?,
                now(h).await?
            )
            .await?
            .is_none()
    );
    let counts:Vec<(Vec<u8>,i64)>=sqlx::query_as("SELECT uuid_send(channel_id),count(*) FROM game_world_raid_creatures WHERE firing_id=encode($1,'hex')::uuid GROUP BY channel_id ORDER BY channel_id")
        .bind(fixture::id(95).as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(counts.len(), 2);
    assert!(counts.iter().all(|(_, count)| *count == 9));
    drop(authority);
    drop(seal);
    Ok(())
}
