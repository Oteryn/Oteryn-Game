//! T94 PostgreSQL proof runs on its own retained-harness database, never a mock pool.
use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, debug, runtime};
use crate::durability::world_event_outcomes::{Disposition, EventBinding};
use crate::native_admission_source::world_event_outcomes::{Snapshot, digest};
use sqlx::Row;

#[test]
fn t94_postgres_persistence_replay_fencing_and_commit_boundaries() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(async {
                    let start = std::time::Instant::now();
                    let h = Harness::create(admin, "t94_authority", false).await?;
                    let result = case(&h).await;
                    h.cleanup().await?;
                    eprintln!(
                        "T94_PG_EXECUTED persistence/replay/fencing elapsed_ms={}",
                        start.elapsed().as_millis()
                    );
                    result
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "T94 thread failed")?
        .map_err(Into::into)
}
async fn case(h: &Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    assert_eq!(h.root.t94_produce(&authority).await?, Disposition::Pending);
    assert!(
        h.root
            .t94_next(&authority, "world_event_outcome")
            .await?
            .is_none()
    );
    assert!(
        h.root
            .t94_initialize(&authority, "oteryn:game:primary", &mut TestFence(1))
            .await
            .is_err(),
        "a published fence cannot initialize an empty restored cache"
    );
    assert_eq!(
        h.root
            .t94_initialize(&authority, "oteryn:game:primary", &mut TestFence(0))
            .await?,
        Disposition::Applied
    );
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/contracts/fixtures/world-event-guild-season/snapshots.json"
    ))?;
    let raw = vectors["vectors"][0]["wire"].as_str().ok_or("wire")?;
    let Snapshot::Event(mut initial) = Snapshot::decode(raw)? else {
        return Err("fixture".into());
    };
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    initial.event[0].lock_at = (now + 3).to_string();
    initial.event[0].scheduled_start = (now + 4).to_string();
    initial.event[0].scheduled_end = (now + 5).to_string();
    let question = initial.event[0].clone();
    let binding = EventBinding {
        channel_policy: Default::default(),
        occurrence_id: question.occurrence_id.clone(),
        withdraw_display: false,
        question_key: "fixture:raid".into(),
        question: question.clone(),
        channels: Vec::new(),
        advance_approved: true,
        outcomes: std::collections::BTreeMap::from([(
            "source-unavailable".into(),
            "completed".into(),
        )]),
        void_reason: Some("SOURCE_UNAVAILABLE".into()),
        corrections_closed: true,
        correction_deadline: Some((now + 6) as u64),
    };
    sqlx::query("UPDATE game_t94_control SET enabled=true,reconciled=true,recovery_generation=1,resync_complete=true,projection_epoch=3,source_authority='oteryn:game:primary'").execute(&h.pool).await?;
    // No accepted native source: even an approved question cannot fabricate a run.
    sqlx::query("INSERT INTO game_t94_bindings(family,world_id,subject_id,approved,binding,checked_at) VALUES('world_event_outcome',$1::uuid,$2::uuid,true,$3,0)").bind(&initial.world_id).bind(&initial.event_id).bind(serde_json::to_vec(&binding)?).execute(&h.pool).await?;
    assert_eq!(h.root.t94_produce(&authority).await?, Disposition::Pending);
    assert!(
        h.root
            .t94_next(&authority, "world_event_outcome")
            .await?
            .is_none()
    );
    // Test-native evidence is independently seeded, then interpreted by the real authority.
    let fact = serde_json::json!({"world":initial.world_id,"kind":"WORLD_CHANGE","occurrence":question.occurrence_id,"key":question.source_key,"channel":"","phase":"admitted","cut":"fixture/1","generation":1,"native_at":now,"proof":digest(b"native fixture"),"content_sha256":question.content_sha256});
    let mut binding = binding;
    binding.question.kind = "DAILY_WORLD_GATE".into();
    let mut fact = fact;
    fact["kind"] = "DAILY_WORLD_GATE".into();
    let receipt = serde_json::to_vec(&fact)?;
    sqlx::query("INSERT INTO game_t94_native_receipts(world_id,kind,occurrence_id,source_key,source_cut,recovery_generation,receipt,receipt_sha256,observed_at) VALUES($1::uuid,'DAILY_WORLD_GATE',$2::uuid,$3,'fixture/1',1,$4,sha256($4),$5)")
  .bind(&initial.world_id).bind(&question.occurrence_id).bind(&question.source_key).bind(&receipt).bind(now).execute(&h.pool).await?;
    sqlx::query("UPDATE game_t94_bindings SET binding=$1")
        .bind(serde_json::to_vec(&binding)?)
        .execute(&h.pool)
        .await?;
    assert_eq!(h.root.t94_produce(&authority).await?, Disposition::Applied);
    let first = h
        .root
        .t94_next(&authority, "world_event_outcome")
        .await?
        .ok_or("publication")?;
    assert_eq!(
        h.root.t94_next(&authority, "world_event_outcome").await?,
        Some(first.clone())
    );
    assert_eq!(h.root.t94_produce(&authority).await?, Disposition::Pending);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_t94_publications")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 1);
    assert!(sqlx::query("UPDATE game_t94_publications SET wire='changed'::bytea,wire_sha256=sha256('changed'::bytea)").execute(&h.pool).await.is_err());
    // A new root/connection and independently opened sealed authority recover
    // committed bytes without using the producer's previous process memory.
    let restarted = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
    assert!(restarted.maintain_ready_once().await?);
    let restarted_authority = restarted
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    assert_eq!(
        restarted
            .t94_next(&restarted_authority, "world_event_outcome")
            .await?,
        Some(first.clone())
    );
    // Real clock crosses lock; LOCKED must commit before a native terminal source.
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    sqlx::query("CREATE FUNCTION t94_reject_publication() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected T94 commit failure' USING ERRCODE='23514'; END $$").execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER t94_reject_publication BEFORE INSERT ON game_t94_publications FOR EACH ROW EXECUTE FUNCTION t94_reject_publication()").execute(&h.pool).await?;
    assert!(h.root.t94_produce(&authority).await.is_err());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_t94_publications")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 1);
    sqlx::query("DROP TRIGGER t94_reject_publication ON game_t94_publications")
        .execute(&h.pool)
        .await?;
    assert_eq!(h.root.t94_produce(&authority).await?, Disposition::Applied);
    let locked_wire: Vec<u8> = sqlx::query_scalar(
        "SELECT wire FROM game_t94_publications ORDER BY delivery_order DESC LIMIT 1",
    )
    .fetch_one(&h.pool)
    .await?;
    let Snapshot::Event(locked) = Snapshot::decode(std::str::from_utf8(&locked_wire)?)? else {
        return Err("locked".into());
    };
    assert_eq!(locked.event[0].state, "LOCKED");
    // Administrative interpretation is fixed before publication: the binding cannot be rewritten.
    assert!(
        sqlx::query("UPDATE game_t94_bindings SET binding=convert_to('{}','UTF8')")
            .execute(&h.pool)
            .await
            .is_err()
    );
    // Source loss is a real accepted receipt. It cannot become a success.
    let mut lost = fact.clone();
    lost["phase"] = "source-unavailable".into();
    lost["cut"] = "fixture/2".into();
    let loss_at: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    lost["native_at"] = loss_at.into();
    let lost_bytes = serde_json::to_vec(&lost)?;
    sqlx::query("INSERT INTO game_t94_native_receipts(world_id,kind,occurrence_id,source_key,source_cut,recovery_generation,receipt,receipt_sha256,observed_at) VALUES($1::uuid,'DAILY_WORLD_GATE',$2::uuid,$3,'fixture/2',1,$4,sha256($4),floor(extract(epoch FROM statement_timestamp()))::bigint)")
 .bind(&initial.world_id).bind(&question.occurrence_id).bind(&question.source_key).bind(&lost_bytes).execute(&h.pool).await?;
    assert_eq!(h.root.t94_produce(&authority).await?, Disposition::Applied);
    let terminal: Vec<u8> = sqlx::query_scalar(
        "SELECT wire FROM game_t94_publications ORDER BY delivery_order DESC LIMIT 1",
    )
    .fetch_one(&h.pool)
    .await?;
    let Snapshot::Event(terminal) = Snapshot::decode(std::str::from_utf8(&terminal)?)? else {
        return Err("terminal".into());
    };
    assert_eq!(terminal.event[0].state, "VOID");
    assert!(terminal.event[0].outcome.is_empty());
    assert!(h.root.t94_next_audit(&authority).await?.is_some());
    // A fresh source receipt yields a correction, bound to the immediate prior record.
    lost = fact.clone();
    lost["phase"] = "source-unavailable".into();
    lost["cut"] = "fixture/3".into();
    lost["native_at"] = loss_at.into();
    let correction = serde_json::to_vec(&lost)?;
    sqlx::query("INSERT INTO game_t94_native_receipts(world_id,kind,occurrence_id,source_key,source_cut,recovery_generation,receipt,receipt_sha256,observed_at) VALUES($1::uuid,'DAILY_WORLD_GATE',$2::uuid,$3,'fixture/3',1,$4,sha256($4),floor(extract(epoch FROM statement_timestamp()))::bigint)").bind(&initial.world_id).bind(&question.occurrence_id).bind(&question.source_key).bind(correction).execute(&h.pool).await?;
    assert_eq!(h.root.t94_produce(&authority).await?, Disposition::Applied);
    let corrected: Vec<u8> = sqlx::query_scalar(
        "SELECT wire FROM game_t94_publications ORDER BY delivery_order DESC LIMIT 1",
    )
    .fetch_one(&h.pool)
    .await?;
    let Snapshot::Event(corrected) = Snapshot::decode(std::str::from_utf8(&corrected)?)? else {
        return Err("correction".into());
    };
    assert_eq!(
        corrected.event[0].correction_of,
        vec![digest(&serde_json::to_vec(&terminal.event[0])?)]
    );
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    sqlx::query("UPDATE game_t94_control SET complete_through=floor(extract(epoch FROM statement_timestamp()))::bigint").execute(&h.pool).await?;
    assert_eq!(h.root.t94_produce(&authority).await?, Disposition::Applied);
    let closed: Vec<u8> = sqlx::query_scalar(
        "SELECT wire FROM game_t94_publications ORDER BY delivery_order DESC LIMIT 1",
    )
    .fetch_one(&h.pool)
    .await?;
    let Snapshot::Event(closed) = Snapshot::decode(std::str::from_utf8(&closed)?)? else {
        return Err("closed".into());
    };
    assert_eq!(closed.event[0].state, "CLOSED");
    assert_eq!(h.root.t94_produce(&authority).await?, Disposition::Pending);
    // Reopen reads immutable committed bytes; a failed ACK leaves the row pending.
    let claimed = h
        .root
        .t94_claim(&authority, "world_event_outcome")
        .await?
        .ok_or("claimed publication")?;
    assert!(
        h.root
            .t94_claim(&authority, "world_event_outcome")
            .await?
            .is_none()
    );
    assert!(
        h.root
            .t94_delivery_current(
                &authority,
                "world_event_outcome",
                claimed.epoch,
                claimed.delivery_token.as_deref().ok_or("token")?
            )
            .await?
    );
    let mut stale = claimed.clone();
    stale.delivery_token = None;
    assert!(!h.root.t94_ack(&authority, &stale).await?);
    assert!(h.root.t94_ack(&authority, &claimed).await?);
    h.root.t94_release(&authority, &claimed).await?;
    let locked = h
        .root
        .t94_claim(&authority, "world_event_outcome")
        .await?
        .ok_or("locked pending")?;
    assert_eq!(locked.revision, 2);
    assert!(h.root.t94_ack(&authority, &locked).await?);
    h.root.t94_release(&authority, &locked).await?;
    while let Some(p) = h.root.t94_claim(&authority, "world_event_outcome").await? {
        assert!(h.root.t94_ack(&authority, &p).await?);
        h.root.t94_release(&authority, &p).await?;
    }
    assert!(
        h.root
            .t94_next(&authority, "world_event_outcome")
            .await?
            .is_none()
    );
    assert!(
        restarted
            .t94_next(&restarted_authority, "world_event_outcome")
            .await?
            .is_none()
    );
    eprintln!("T94_PHASE lifecycle_committed");
    // Full immutable lineage, including LOCKED and corrections, must stage in a new epoch.
    let request: String = sqlx::query_scalar("SELECT uuidv7()::text")
        .fetch_one(&h.pool)
        .await?;
    let mut fence = TestFence(3);
    assert_eq!(
        h.root
            .t94_begin_resync(&authority, &request, &mut fence)
            .await
            .map_err(|e| format!("T94 begin resync: {e:?}"))?,
        4
    );
    assert_eq!(fence.0, 4);
    assert_eq!(
        h.root
            .t94_begin_resync(&authority, &request, &mut fence)
            .await
            .map_err(|e| format!("T94 begin resync: {e:?}"))?,
        4
    );
    for _ in 0..8 {
        h.root
            .t94_produce(&authority)
            .await
            .map_err(|e| format!("T94 resync page: {e:?}"))?;
    }
    assert!(
        h.root
            .t94_watermark(&authority, "world_event_outcome")
            .await?
            .is_none()
    );
    let staged: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_t94_publications WHERE projection_epoch=4 AND resync_member",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(staged, 5);
    while let Some(p) = h.root.t94_claim(&authority, "world_event_outcome").await? {
        let old:Vec<u8>=sqlx::query_scalar("SELECT wire FROM game_t94_publications WHERE projection_epoch=3 AND projection_revision=$1::text::numeric").bind(p.revision.to_string()).fetch_one(&h.pool).await?;
        let mut original = Snapshot::decode(std::str::from_utf8(&old)?)?;
        let Snapshot::Event(s) = &mut original else {
            return Err("resync family".into());
        };
        s.projection_epoch = "4".into();
        assert_eq!(p.wire, original.encode()?);
        assert!(h.root.t94_ack(&authority, &p).await?);
        h.root.t94_release(&authority, &p).await?;
    }
    sqlx::query("UPDATE game_t94_control SET complete_through=floor(extract(epoch FROM statement_timestamp()))::bigint").execute(&h.pool).await?;
    assert!(
        h.root
            .t94_watermark(&authority, "world_event_outcome")
            .await?
            .is_some()
    );
    let stored=sqlx::query("SELECT wire,wire_sha256,acknowledged FROM game_t94_publications ORDER BY delivery_order LIMIT 1").fetch_one(&h.pool).await?;
    assert_eq!(stored.try_get::<Vec<u8>, _>("wire")?, first.wire.as_bytes());
    assert!(stored.try_get::<bool, _>("acknowledged")?);

    eprintln!("T94_PHASE resync_acknowledged");
    // Withdrawal changes display, never the retained final authority record.
    sqlx::query("UPDATE game_t94_bindings SET binding=convert_to(jsonb_set(convert_from(binding,'UTF8')::jsonb,'{withdraw_display}','true'::jsonb)::text,'UTF8')").execute(&h.pool).await?;
    assert_eq!(h.root.t94_produce(&authority).await?, Disposition::Applied);
    let withdrawn = h
        .root
        .t94_next(&authority, "world_event_outcome")
        .await?
        .ok_or("withdrawn display")?;
    let Snapshot::Event(withdrawn) = Snapshot::decode(&withdrawn.wire)? else {
        return Err("withdrawal family".into());
    };
    assert!(withdrawn.event.is_empty());
    // P90D is payload expiry, not permission to reuse the native question.
    sqlx::query("UPDATE game_t94_subjects SET final_at=floor(extract(epoch FROM statement_timestamp()))::bigint-7776001").execute(&h.pool).await?;
    sqlx::query("UPDATE game_t94_native_receipts SET observed_at=floor(extract(epoch FROM statement_timestamp()))::bigint-7776001").execute(&h.pool).await?;
    h.root
        .t94_expire(&authority)
        .await
        .map_err(|e| format!("T94 expiry: {e:?}"))?;
    let remaining:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_t94_publications),(SELECT count(*) FROM game_t94_subjects WHERE final_state AND octet_length(record)=0),(SELECT count(*) FROM game_t94_native_receipts WHERE octet_length(receipt)=0 AND expired_at IS NOT NULL)").fetch_one(&h.pool).await?;
    assert_eq!(remaining, (0, 1, 3));
    assert_eq!(h.root.t94_produce(&authority).await?, Disposition::Pending);
    season_case(h, &authority)
        .await
        .map_err(|e| format!("T94 season: {e:?}"))?;
    sqlx::query("UPDATE game_t94_control SET recovery_generation=2")
        .execute(&h.pool)
        .await?;
    assert!(h.root.t94_produce(&authority).await.is_err());
    assert!(
        h.root
            .t94_next(&authority, "world_event_outcome")
            .await
            .is_err()
    );
    Ok(())
}

struct TestFence(u64);
impl crate::native_admission_source::account_characters::EpochFence for TestFence {
    fn read(
        &mut self,
    ) -> Result<u64, crate::native_admission_source::account_characters::FenceUnusable> {
        Ok(self.0)
    }
    fn persist(
        &mut self,
        epoch: u64,
    ) -> Result<(), crate::native_admission_source::account_characters::FenceUnusable> {
        self.0 = epoch;
        Ok(())
    }
}

async fn season_case(
    h: &Harness,
    a: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    use crate::durability::world_event_outcomes::{Contribution, SeasonBinding};
    use crate::native_admission_source::world_event_outcomes::{Challenge, SeasonRecord};
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let (world, season, challenge, guild, occurrence): (String, String, String, String, String) =
        sqlx::query_as(
            "SELECT uuidv7()::text,uuidv7()::text,uuidv7()::text,uuidv7()::text,uuidv7()::text",
        )
        .fetch_one(&h.pool)
        .await?;
    // Only seed a preexisting Guild under the isolated harness administrator.
    let mut guild_seed = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *guild_seed)
        .await?;
    sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at) VALUES($1::uuid,$2::uuid,'Outcome Guild',2,$3,1,$3)").bind(&guild).bind(&world).bind(now).execute(&mut *guild_seed).await?;
    guild_seed.commit().await?;
    let b = SeasonBinding {
        attribution: Default::default(),
        withdraw_display: false,
        season: SeasonRecord {
            ruleset_revision: "t94:pg-native-v1".into(),
            starts_at: (now + 2).to_string(),
            ends_at: (now + 4).to_string(),
            freeze_at: (now + 5).to_string(),
            state: "OPEN".into(),
            challenges: vec![Challenge {
                challenge_id: challenge.clone(),
                source_key: "oteryn:creature.rat".into(),
                scoring_revision: "t94:pg-native-v1".into(),
                max_score: "100".into(),
            }],
            standings: vec![],
            computed_at: now.to_string(),
            source_complete_through: now.to_string(),
            correction_of: vec![],
        },
        rules: std::collections::BTreeMap::from([("oteryn:creature.rat".into(), 1)]),
        eligible_guilds: vec![guild.clone()],
        tombstone_name: None,
        corrections_closed: false,
        correction_deadline: Some((now + 5) as u64),
        complete_through: now as u64,
    };
    sqlx::query("INSERT INTO game_t94_bindings(family,world_id,subject_id,approved,approved_at,binding) VALUES('guild_season',$1::uuid,$2::uuid,true,to_timestamp($3::bigint),$4)").bind(&world).bind(&season).bind(now-3).bind(serde_json::to_vec(&b)?).execute(&h.pool).await?;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let accepted: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let source:String=sqlx::query_scalar("INSERT INTO game_t94_native_receipts(world_id,kind,occurrence_id,source_key,source_cut,recovery_generation,receipt,receipt_sha256,observed_at) VALUES($1::uuid,'GUILD_SOURCE',$2::uuid,'oteryn:creature.rat','pg/source',1,'native'::bytea,sha256('native'::bytea),$3) RETURNING receipt_id::text").bind(&world).bind(&occurrence).bind(accepted).fetch_one(&h.pool).await?;
    sqlx::query("INSERT INTO game_t94_contributions(season_id,challenge_id,occurrence_id,guild_id,world_id,score,guild_name,membership_cut,source_receipt_id,revision,receipt_sha256) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5::uuid,1,'Outcome Guild',sha256('membership'::bytea),$6::uuid,1,sha256('receipt'::bytea))").bind(&season).bind(&challenge).bind(&occurrence).bind(&guild).bind(&world).bind(&source).execute(&h.pool).await?;
    assert_eq!(h.root.t94_produce(a).await?, Disposition::Applied);
    let _ = h.root.t94_watermark(a, "guild_season").await?;
    assert!(sqlx::query("UPDATE game_t94_bindings SET approved_at=approved_at-interval '1 day' WHERE family='guild_season' AND subject_id=$1::uuid").bind(&season).execute(&h.pool).await.is_err());
    let first = latest_season(h, &season).await?;
    assert_eq!(first.season[0].standings[0].total_score, "1");
    let correction = Contribution {
        season_id: season.clone(),
        challenge_id: challenge,
        occurrence_id: occurrence,
        guild_id: guild,
        source_receipt_id: source,
        revision: 2,
        withdraw: true,
    };
    let request:String=sqlx::query_scalar("INSERT INTO game_t94_corrections(request,approved) VALUES($1,true) RETURNING request_id::text").bind(serde_json::to_vec(&correction)?).fetch_one(&h.pool).await?;
    assert_eq!(h.root.t94_produce(a).await?, Disposition::Applied);
    assert_eq!(h.root.t94_produce(a).await?, Disposition::Applied);
    let corrected = latest_season(h, &season).await?;
    assert_eq!(corrected.season[0].standings[0].total_score, "0");
    assert_eq!(
        corrected.season[0].correction_of,
        vec![digest(&serde_json::to_vec(&first.season[0])?)]
    );
    let receipt: Vec<u8> =
        sqlx::query_scalar("SELECT receipt FROM game_t94_corrections WHERE request_id=$1::uuid")
            .bind(&request)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(h.root.t94_produce(a).await?, Disposition::Pending);
    let replay: Vec<u8> =
        sqlx::query_scalar("SELECT receipt FROM game_t94_corrections WHERE request_id=$1::uuid")
            .bind(&request)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(receipt, replay);
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    sqlx::query("UPDATE game_t94_bindings SET binding=convert_to(jsonb_set(jsonb_set(convert_from(binding,'UTF8')::jsonb,'{corrections_closed}','true'::jsonb),'{complete_through}',to_jsonb(floor(extract(epoch FROM statement_timestamp()))::bigint))::text,'UTF8') WHERE family='guild_season' AND subject_id=$1::uuid").bind(&season).execute(&h.pool).await?;
    assert_eq!(h.root.t94_produce(a).await?, Disposition::Applied);
    let frozen = latest_season(h, &season).await?;
    assert_eq!(frozen.season[0].state, "FROZEN");
    let late:String=sqlx::query_scalar("INSERT INTO game_t94_corrections(request,approved) VALUES($1,true) RETURNING request_id::text").bind(serde_json::to_vec(&correction)?).fetch_one(&h.pool).await?;
    assert_eq!(h.root.t94_produce(a).await?, Disposition::Applied);
    let refused: Vec<u8> =
        sqlx::query_scalar("SELECT receipt FROM game_t94_corrections WHERE request_id=$1::uuid")
            .bind(late)
            .fetch_one(&h.pool)
            .await?;
    assert!(std::str::from_utf8(&refused)?.contains("REFUSED_FROZEN"));
    assert_eq!(latest_season(h, &season).await?, frozen);
    assert!(h.root.t94_watermark(a, "guild_season").await?.is_some());
    let mut disband_fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *disband_fixture)
        .await?;
    sqlx::query("UPDATE game_guilds SET state=4 WHERE guild_id=$1::uuid")
        .bind(&correction.guild_id)
        .execute(&mut *disband_fixture)
        .await?;
    disband_fixture.commit().await?;
    assert!(
        h.root.t94_watermark(a, "guild_season").await?.is_none(),
        "unpublished privacy withdrawal holds completeness"
    );
    assert_eq!(h.root.t94_produce(a).await?, Disposition::Applied);
    assert!(
        latest_season(h, &season).await?.season.is_empty(),
        "withdrawal is not zero scores or VOID"
    );
    Ok(())
}
async fn latest_season(
    h: &Harness,
    season: &str,
) -> TestResult<crate::native_admission_source::world_event_outcomes::SeasonSnapshot> {
    let raw:Vec<u8>=sqlx::query_scalar("SELECT wire FROM game_t94_publications WHERE subject_id=$1::uuid ORDER BY delivery_order DESC LIMIT 1").bind(season).fetch_one(&h.pool).await?;
    let Snapshot::Season(s) = Snapshot::decode(std::str::from_utf8(&raw)?)? else {
        return Err("season family".into());
    };
    Ok(s)
}
