//! PUBPROJ-C real PostgreSQL proofs; no clock or metadata-return inference.
use crate::bestiary_postgres_harness::{
    Harness, TestResult, WORLD, configured_admin, debug, id, runtime,
};
use oteryn_gameplay_content::content::ProductionKey;
use crate::durability::raid_schedule::{self as store, RaidWrite};
use crate::foundation::WorldId;
use crate::native_admission_source::public_projections::v2::{Body, Family};
use oteryn_gameplay_encounter_rules::raid::{
    Announcement, AnnouncementKind, CalendarDay, CalendarSelector, RaidAction, RaidDefinition,
    RaidPolicy, RaidSchedule, Wave, WaveEntry,
};
use sqlx::{Postgres, Row, Transaction};

fn run_case(
    tag: &'static str,
    case: impl for<'a> FnOnce(
        &'a Harness,
    )
        -> std::pin::Pin<Box<dyn std::future::Future<Output = TestResult> + 'a>>
    + Send
    + 'static,
) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(async move {
                    let start = std::time::Instant::now();
                    let h = Harness::create(admin, tag, false).await?;
                    let result = case(&h).await;
                    h.cleanup().await?;
                    eprintln!(
                        "PUBPROJ_WORLD_DAILY_PG_EXECUTED case={tag} elapsed_ms={}",
                        start.elapsed().as_millis()
                    );
                    result
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "world_daily thread failed")?
        .map_err(Into::into)
}
async fn begin(h: &Harness) -> TestResult<Transaction<'_, Postgres>> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}
async fn start(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT floor(extract(epoch FROM ((date_trunc('day',(clock_timestamp() AT TIME ZONE 'Europe/Berlin')-interval '6 hours')+interval '6 hours') AT TIME ZONE 'Europe/Berlin')))::bigint").fetch_one(&h.pool).await?)
}
async fn seed(tx: &mut Transaction<'_, Postgres>, at: i64) -> TestResult {
    for (epoch, from) in [(0_i64, at - 31 * 86400), (1, at - 86400), (2, at)] {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::bigint,decode(repeat('07',32),'hex'),'pubproj-daily-map',decode(repeat('08',32),'hex'),decode(repeat('09',32),'hex'),decode(repeat('0a',32),'hex'),'TIBIA_SERVER_SAVE_BERLIN_06',1,timestamptz 'epoch'+$3*interval '1 second',to_char(((timestamptz 'epoch'+$3*interval '1 second') AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM'))")
            .bind(id(WORLD).as_slice()).bind(epoch).bind(from).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_world_daily_boosts(world_id,reset_epoch,creature_key,boss_key) VALUES(encode($1,'hex')::uuid,$2::bigint,'monster:dragon','boss:morgaroth')")
            .bind(id(WORLD).as_slice()).bind(epoch).execute(&mut **tx).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,2)")
        .bind(id(WORLD).as_slice())
        .execute(&mut **tx)
        .await?;
    Ok(())
}
fn definition(policy: RaidPolicy) -> TestResult<RaidDefinition> {
    Ok(RaidDefinition {
        policy,
        despawn_after_ms: Some(1000),
        announcements: vec![Announcement {
            delay_ms: 1000,
            text: "A raid approaches.".into(),
            kind: AnnouncementKind::Warning,
        }],
        waves: vec![Wave {
            delay_ms: 2000,
            entries: vec![WaveEntry {
                creature: ProductionKey::new("creature:rat").map_err(debug)?,
                count: 1,
                area_anchor: ProductionKey::new("area:raid").map_err(debug)?,
            }],
        }],
    })
}
#[test]
fn world_daily_postgres_dst_and_source_rollback_revision_ack_race_resync() -> TestResult {
    run_case("pubproj_world_daily", |h| {
        Box::pin(async move {
            // PostgreSQL's same authoritative calendar produces23h/25h, not86400.
            let spans:(i64,i64)=sqlx::query_as("SELECT game_public_world_daily_next_slot('2026-03-28 06:00 Europe/Berlin','TIBIA_SERVER_SAVE_BERLIN_06',1)-extract(epoch FROM timestamptz '2026-03-28 06:00 Europe/Berlin')::bigint,game_public_world_daily_next_slot('2026-10-24 06:00 Europe/Berlin','TIBIA_SERVER_SAVE_BERLIN_06',1)-extract(epoch FROM timestamptz '2026-10-24 06:00 Europe/Berlin')::bigint").fetch_one(&h.pool).await?;
            assert_eq!(spans, (23 * 3600, 25 * 3600));
            let at = start(h).await?;
            let mut tx = h.pool.begin().await?;
            seed(&mut tx, at).await?;
            let pending: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_public_v2_outbox WHERE family='world_daily'",
            )
            .fetch_one(&mut *tx)
            .await?;
            assert!(
                pending > 0,
                "source transaction includes revision/outbox before commit"
            );
            tx.rollback().await?;
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_public_v2_heads WHERE family='world_daily'"
                )
                .fetch_one(&h.pool)
                .await?,
                0,
                "source rollback includes revision/outbox"
            );
            let mut tx = h.pool.begin().await?;
            seed(&mut tx, at).await?;
            tx.commit().await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let recovery = authority.record_for(&h.root).map_err(debug)?;
            let first = h
                .root
                .next_public_v2(&authority, Family::WorldDaily, &[])
                .await
                .map_err(debug)?
                .ok_or("daily source snapshot")?;
            let Body::WorldDaily(rows) = &first.body else {
                return Err("world_daily body".into());
            };
            assert_eq!(
                rows[0].history.len(),
                1,
                "history excludes source older than30days"
            );
            assert_eq!(rows[0].history_complete_from, (at - 86400).to_string());
            assert!(rows[0].raids.is_empty());
            let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
            let key = ProductionKey::new("raid:public-fixed").map_err(debug)?;
            let d = definition(RaidPolicy::Fixed {
                selector: CalendarSelector::Weekday(1),
                offset_after_reset_ms: 0,
            })?;
            let mut tx = begin(h).await?;
            assert_eq!(
                store::initialize(&mut tx, &recovery, world, &key, 2, at * 1000 - 60_000).await?,
                RaidWrite::Applied
            );
            tx.commit().await?;
            let initial = RaidSchedule::initial(2, at * 1000 - 60_000)?;
            let running = initial.plan(
                world,
                &key,
                &d,
                RaidAction::Check {
                    now_ms: at * 1000,
                    eligible: true,
                    calendar: Some(CalendarDay {
                        reset_at_ms: at * 1000,
                        weekday: 1,
                        month_day: 1,
                    }),
                    firing_id: id(150),
                    bundle_digest: [7; 32],
                },
            )?;
            let mut tx = begin(h).await?;
            assert_eq!(
                store::compare_and_set(&mut tx, &recovery, &running).await?,
                RaidWrite::Applied
            );
            tx.commit().await?;
            let observed = h
                .root
                .next_public_v2(&authority, Family::WorldDaily, &[])
                .await
                .map_err(debug)?
                .ok_or("committed fixed raid")?;
            let Body::WorldDaily(rows) = &observed.body else {
                return Err("daily body".into());
            };
            assert_eq!(rows[0].raids.len(), 1);
            assert_eq!(rows[0].raids[0].scheduled_at, at.to_string());
            let done = running.after().plan(
                world,
                &key,
                &d,
                RaidAction::Close {
                    now_ms: at * 1000 + 1000,
                },
            )?;
            let mut tx = begin(h).await?;
            assert_eq!(
                store::compare_and_set(&mut tx, &recovery, &done).await?,
                RaidWrite::Applied
            );
            tx.commit().await?;
            h.root
                .ack_public_v2(&authority, &observed)
                .await
                .map_err(debug)?;
            let later = h
                .root
                .next_public_v2(&authority, Family::WorldDaily, &[])
                .await
                .map_err(debug)?
                .ok_or("newer done revision survives running ACK")?;
            assert!(later.projection_revision > observed.projection_revision);
            let Body::WorldDaily(rows) = &later.body else {
                return Err("daily body".into());
            };
            assert_eq!(rows[0].raids[0].state,crate::native_admission_source::public_projections::v2::world_daily::RaidState::Done);
            // A second native fixed firing may be cancelled, retaining its slot.
            let cancelled_key = ProductionKey::new("raid:cancelled-fixed").map_err(debug)?;
            let mut tx = begin(h).await?;
            store::initialize(
                &mut tx,
                &recovery,
                world,
                &cancelled_key,
                2,
                at * 1000 - 60_000,
            )
            .await?;
            tx.commit().await?;
            let cancelled_running = initial.plan(
                world,
                &cancelled_key,
                &d,
                RaidAction::Check {
                    now_ms: at * 1000,
                    eligible: true,
                    calendar: Some(CalendarDay {
                        reset_at_ms: at * 1000,
                        weekday: 1,
                        month_day: 1,
                    }),
                    firing_id: id(152),
                    bundle_digest: [7; 32],
                },
            )?;
            let mut tx = begin(h).await?;
            store::compare_and_set(&mut tx, &recovery, &cancelled_running).await?;
            tx.commit().await?;
            let cancelled =
                cancelled_running
                    .after()
                    .plan(world, &cancelled_key, &d, RaidAction::Retire)?;
            let mut tx = begin(h).await?;
            store::compare_and_set(&mut tx, &recovery, &cancelled).await?;
            tx.commit().await?;
            let cancelled_snapshot = h
                .root
                .next_public_v2(&authority, Family::WorldDaily, &[])
                .await
                .map_err(debug)?
                .ok_or("cancelled fixed firing")?;
            let Body::WorldDaily(records) = &cancelled_snapshot.body else {
                return Err("daily body".into());
            };
            assert!(records[0].raids.iter().any(|r|r.raid_key==cancelled_key.as_str() && r.state==crate::native_admission_source::public_projections::v2::world_daily::RaidState::Cancelled));
            // A committed random firing remains private and changes no public revision.
            let random_key = ProductionKey::new("raid:private-random").map_err(debug)?;
            let random_d = definition(RaidPolicy::Random {
                mean_interval_s: 1,
                min_gap_s: 0,
            })?;
            let mut tx = begin(h).await?;
            store::initialize(
                &mut tx,
                &recovery,
                world,
                &random_key,
                2,
                at * 1000 - 60_000,
            )
            .await?;
            tx.commit().await?;
            let random = initial.plan(
                world,
                &random_key,
                &random_d,
                RaidAction::Check {
                    now_ms: at * 1000,
                    eligible: true,
                    calendar: None,
                    firing_id: id(153),
                    bundle_digest: [7; 32],
                },
            )?;
            assert!(random.after().latest_firing.is_some());
            let mut tx = begin(h).await?;
            store::compare_and_set(&mut tx, &recovery, &random).await?;
            tx.commit().await?;
            let private_unchanged = h
                .root
                .next_public_v2(&authority, Family::WorldDaily, &[])
                .await
                .map_err(debug)?
                .ok_or("pending fixed facts")?;
            assert_eq!(
                private_unchanged.projection_revision,
                cancelled_snapshot.projection_revision
            );
            let Body::WorldDaily(records) = &private_unchanged.body else {
                return Err("daily body".into());
            };
            assert!(
                !records[0]
                    .raids
                    .iter()
                    .any(|r| r.raid_key == random_key.as_str())
            );
            h.root
                .ack_public_v2(&authority, &private_unchanged)
                .await
                .map_err(debug)?;
            sqlx::query("SELECT game_public_projection_resync('world_daily',false)")
                .execute(&h.pool)
                .await?;
            let replay = h
                .root
                .next_public_v2(&authority, Family::WorldDaily, &[])
                .await
                .map_err(debug)?
                .ok_or("operator world_daily resync")?;
            assert!(replay.projection_revision > private_unchanged.projection_revision);
            // Metadata for a future check is not a new public raid or a revision.
            sqlx::query("UPDATE game_world_raid_schedules SET revision=revision+1,next_check_at_ms=greatest(next_check_at_ms+60000,floor(extract(epoch FROM clock_timestamp())*1000)::bigint+86400000) WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
            .bind(id(WORLD).as_slice()).bind(key.as_str()).execute(&h.pool).await?;
            let unchanged = h
                .root
                .next_public_v2(&authority, Family::WorldDaily, &[])
                .await
                .map_err(debug)?
                .ok_or("existing resync")?;
            assert_eq!(unchanged.projection_revision, replay.projection_revision);
            Ok(())
        })
    })
}
#[test]
fn world_daily_postgres_unknown_current_firing_withholds_instead_of_fabricating_empty() -> TestResult
{
    run_case("pubproj_daily_unknown", |h| {
        Box::pin(async move {
            let at = start(h).await?;
            let mut tx = h.pool.begin().await?;
            seed(&mut tx, at).await?;
            tx.commit().await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut tx = h.pool.begin().await?;
            // An old producer's committed occurrence has no fixed/nonpublic identity.
            // It remains unknown even if metadata happens to have a plausible name/time.
            sqlx::query("INSERT INTO game_world_raid_schedules(world_id,raid_key,revision,reset_epoch,check_ordinal,next_firing_ordinal,next_check_at_ms,retired) VALUES(encode($1,'hex')::uuid,'raid:legacy-unknown',1,2,0,2,$2,false)")
            .bind(id(WORLD).as_slice()).bind(at*1000+120_000).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_world_raid_schedule_sources(world_id,raid_key,source_kind) VALUES(encode($1,'hex')::uuid,'raid:legacy-unknown',3)").bind(id(WORLD).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_world_raid_firings(world_id,raid_key,ordinal,firing_id,reset_epoch,fired_at_ms,deadline_ms,bundle_digest,state) VALUES(encode($1,'hex')::uuid,'raid:legacy-unknown',1,encode($2,'hex')::uuid,2,$3,$4,decode(repeat('07',32),'hex'),1)")
            .bind(id(WORLD).as_slice()).bind(id(151).as_slice()).bind(at*1000+60_000).bind(at*1000+61_000).execute(&mut *tx).await?;
            tx.commit().await?;
            let row=sqlx::query("SELECT payload,game_public_v2_source_pending('world_daily') AS pending FROM game_public_v2_heads WHERE family='world_daily'").fetch_one(&h.pool).await?;
            assert!(
                row.try_get::<serde_json::Value, _>("payload")?.is_null(),
                "unavailable is not [] withdrawal"
            );
            assert!(row.try_get::<Option<i64>, _>("pending")?.is_some());
            assert!(
                h.root
                    .next_public_v2(&authority, Family::WorldDaily, &[])
                    .await
                    .map_err(debug)?
                    .is_none()
            );
            let facts = h
                .root
                .public_v2_facts(&authority, Family::WorldDaily)
                .await
                .map_err(debug)?;
            assert!(
                facts.oldest_undelivered_ms.is_some_and(|pending| pending <= facts.now_ms),
                "unknown source cannot advance completeness"
            );
            Ok(())
        })
    })
}

#[test]
fn world_daily_postgres_consumed_slots_and_subsecond_fixed_firings_do_not_fabricate_wire()
-> TestResult {
    run_case("pubproj_daily_slots", |h| {
        Box::pin(async move {
            let at = start(h).await?;
            let mut tx = h.pool.begin().await?;
            seed(&mut tx, at).await?;
            tx.commit().await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let recovery = authority.record_for(&h.root).map_err(debug)?;
            let before = h
                .root
                .next_public_v2(&authority, Family::WorldDaily, &[])
                .await
                .map_err(debug)?
                .ok_or("daily baseline")?;
            let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
            let initial = RaidSchedule::initial(2, at * 1000 - 60_000)?;
            let key = ProductionKey::new("raid:blocked-fixed").map_err(debug)?;
            let d = definition(RaidPolicy::Fixed {
                selector: CalendarSelector::Weekday(1),
                offset_after_reset_ms: 0,
            })?;
            let mut tx = begin(h).await?;
            store::initialize(&mut tx, &recovery, world, &key, 2, at * 1000 - 60_000).await?;
            tx.commit().await?;
            let blocked = initial.plan(
                world,
                &key,
                &d,
                RaidAction::Check {
                    now_ms: at * 1000,
                    eligible: false,
                    calendar: Some(CalendarDay {
                        reset_at_ms: at * 1000,
                        weekday: 1,
                        month_day: 1,
                    }),
                    firing_id: id(154),
                    bundle_digest: [7; 32],
                },
            )?;
            assert!(blocked.after().last_fixed_slot_ms.is_some());
            assert!(blocked.after().latest_firing.is_none());
            let mut tx = begin(h).await?;
            store::compare_and_set(&mut tx, &recovery, &blocked).await?;
            tx.commit().await?;
            let unchanged = h
                .root
                .next_public_v2(&authority, Family::WorldDaily, &[])
                .await
                .map_err(debug)?
                .ok_or("same baseline")?;
            assert_eq!(
                unchanged.projection_revision, before.projection_revision,
                "consuming a blocked slot is not a public raid"
            );
            let key = ProductionKey::new("raid:subsecond-fixed").map_err(debug)?;
            let d = definition(RaidPolicy::Fixed {
                selector: CalendarSelector::Weekday(1),
                offset_after_reset_ms: 1,
            })?;
            let mut tx = begin(h).await?;
            store::initialize(&mut tx, &recovery, world, &key, 2, at * 1000 - 60_000).await?;
            tx.commit().await?;
            let subsecond = initial.plan(
                world,
                &key,
                &d,
                RaidAction::Check {
                    now_ms: at * 1000 + 1,
                    eligible: true,
                    calendar: Some(CalendarDay {
                        reset_at_ms: at * 1000,
                        weekday: 1,
                        month_day: 1,
                    }),
                    firing_id: id(155),
                    bundle_digest: [7; 32],
                },
            )?;
            let mut tx = begin(h).await?;
            store::compare_and_set(&mut tx, &recovery, &subsecond).await?;
            tx.commit().await?;
            let classification:(i16,i64)=sqlx::query_as("SELECT public_daily_policy,public_fixed_slot_ms%1000 FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid").bind(id(155).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                classification,
                (2, 1),
                "typed source identity committed before producer refusal"
            );
            assert!(
                h.root
                    .next_public_v2(&authority, Family::WorldDaily, &[])
                    .await
                    .map_err(debug)?
                    .is_none(),
                "a subsecond slot is withheld, never rounded"
            );
            assert!(
                sqlx::query_scalar::<_, Option<i64>>(
                    "SELECT game_public_v2_source_pending('world_daily')"
                )
                .fetch_one(&h.pool)
                .await?
                .is_some()
            );
            Ok(())
        })
    })
}
