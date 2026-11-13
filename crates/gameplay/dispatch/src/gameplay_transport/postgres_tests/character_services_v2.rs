//! G-3 decisions exercise the real reconciled authority on PostgreSQL, with injected time.
use super::TestResult;
use super::fixture::{Harness, debug, id, run};
use crate::native_admission_source::character_commands::{Decision, Input};
use crate::native_admission_source::character_services_v2::CharacterCommandIntentV2;
use serde_json::{Value, json};
use sqlx::Row;
#[test]
fn character_services_v2_postgres_repeatable_rename_delete_cancel_finalize_and_projection()
-> TestResult {
    scenario(false, "char_services_decisions", false)
}
#[test]
fn character_services_v2_postgres_cancel_finalizer_race_has_one_winner() -> TestResult {
    scenario(true, "char_services_race", false)
}
#[test]
fn character_services_v2_postgres_independent_service_and_lcfa_acknowledgements() -> TestResult {
    scenario(false, "char_services_projection", true)
}
fn scenario(race: bool, tag: &'static str, projection_only: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::journal_watchdog::run("character-services-v2", move |stage| {
        run(async move |admin| {
            let h = Harness::create_with_admission_seed(admin, tag, true, false).await?;
            let result: TestResult = async {
            stage.set("fixed clock and authority");
            assert!(!sqlx::query_scalar::<_,bool>("SELECT capture_started FROM game_character_service_feed WHERE singleton").fetch_one(&h.pool).await?);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_service_outbox").fetch_one(&h.pool).await?,0);

            let now: i64 = sqlx::query_scalar("SELECT game_character_service_now()").fetch_one(&h.pool).await?;
            sqlx::query("CREATE TABLE service_test_clock(value BIGINT NOT NULL)").execute(&h.pool).await?;
            sqlx::query("INSERT INTO service_test_clock VALUES($1)").bind(now).execute(&h.pool).await?;
            sqlx::query("GRANT SELECT ON service_test_clock TO oteryn_game_runtime").execute(&h.pool).await?;
            sqlx::query("CREATE OR REPLACE FUNCTION game_character_service_now() RETURNS BIGINT LANGUAGE sql VOLATILE SET search_path=pg_catalog,public AS 'SELECT value FROM service_test_clock'").execute(&h.pool).await?;
            sqlx::query("CREATE OR REPLACE FUNCTION game_character_service_projection_now() RETURNS BIGINT LANGUAGE sql VOLATILE SET search_path=pg_catalog,public AS 'SELECT value FROM service_test_clock'").execute(&h.pool).await?;
            let _clock = super::preadmission::FrozenAdmissionClock::install(&h, now).await?;
            let source = super::house_transfer::RecoverySource::start_at(&h,Some(now as u64)).await?;
            let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            source.evidence.refresh_account(&h.root,&h.node,&account).await.map_err(debug)?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            super::journal_inventory::runtime_role(&h.root).await?;
            let root = sqlx::query("SELECT character_id::text,name,character_revision::text FROM game_character_roots WHERE account_id=$1::uuid").bind(&account).fetch_one(&h.pool).await?;
            let character: String = root.try_get("character_id")?;
            let original_name: String = root.try_get("name")?;
            let mut revision: u64 = root.try_get::<String,_>("character_revision")?.parse()?;
            let raw = intent("rename",1,revision,now,&account,&character,None)?;
            let original_intent=raw.clone();
            stage.set("rename and immutable replay");
            let first = h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter", &operation(1)),Input::Intent(&raw)).await;
            let receipt = committed(first)?;
            assert_eq!(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter",&operation(1)),Input::Intent(&raw)).await,Decision::Publish { receipt: receipt.clone(),result_code:"CHAR_CMD_OK".into() });
            let changed=std::str::from_utf8(&raw)?.replace("Aldric Vale","Aldric View").into_bytes();
            assert_eq!(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter",&operation(1)),Input::Intent(&changed)).await,Decision::Conflict);
            stage.set("independent services revision and acknowledgement");
            use crate::native_admission_source::character_services_projection::Store as _;
            let services_revision:i64=sqlx::query_scalar("SELECT revision FROM game_character_service_revisions WHERE account_id=$1::uuid").bind(&account).fetch_one(&h.pool).await?;
            let lcfa_revision:String=sqlx::query_scalar("SELECT projection_revision::text FROM game_character_account_projections WHERE account_id=$1::uuid").bind(&account).fetch_one(&h.pool).await?;
            let receipt_facts:Value=serde_json::from_slice(&receipt)?;
            assert_eq!(receipt_facts["projection_revision"],json!(services_revision.to_string()));
            assert_ne!(services_revision.to_string(),lcfa_revision);
            let mut delivery=crate::durability::character_services_projection::CharacterServicesStore {root:&h.root,authority:&authority,proof:&h.node};
            let pending_before=delivery.facts().await.map_err(debug)?.oldest_undelivered_ms;
            assert!(pending_before.is_some(),"resync/pending work holds completeness");
            for _ in 0..4 {
                let _=delivery.next().await.map_err(debug)?;
                let phase:i16=sqlx::query_scalar("SELECT phase FROM game_character_service_feed WHERE singleton").fetch_one(&h.pool).await?;
                if phase==3 {break;}
            }
            assert_eq!(sqlx::query_scalar::<_,i16>("SELECT phase FROM game_character_service_feed WHERE singleton").fetch_one(&h.pool).await?,3);
            let snapshot=delivery.next().await.map_err(debug)?.ok_or("services snapshot")?;
            let export=snapshot.encode("oteryn:character-services:primary").map_err(debug)?;
            crate::native_admission_source::character_services_v2::PublishCharacterServiceStateV2::decode(export.as_bytes()).map_err(debug)?;
            let lcfa_pending:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_account_projection_outbox WHERE account_id=$1::uuid").bind(&account).fetch_one(&h.pool).await?;
            delivery.acknowledge(&snapshot).await.map_err(debug)?;
            assert!(sqlx::query_scalar::<_,bool>("SELECT delivered_revision=projection_revision FROM game_character_service_outbox WHERE account_id=$1::uuid AND character_id=$2::uuid").bind(&account).bind(&character).fetch_one(&h.pool).await?);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_account_projection_outbox WHERE account_id=$1::uuid").bind(&account).fetch_one(&h.pool).await?,lcfa_pending);
            let held: (String,i64,i64) = sqlx::query_as("SELECT name_key,released_at,available_at FROM game_character_service_name_holds WHERE character_id=$1::uuid").bind(&character).fetch_one(&h.pool).await?;
            assert_eq!(held.0,original_name.to_ascii_lowercase().replace(' ',"")); assert_eq!(held.2-held.1,30*86400);
            sqlx::query("SELECT game_character_service_release_name($1)").bind(&held.0).execute(&h.pool).await?;
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_name_reservations WHERE name_key=$1").bind(&held.0).fetch_one(&h.pool).await?,1);
            let world: String=sqlx::query_scalar("SELECT world_id::text FROM game_character_roots WHERE character_id=$1::uuid").bind(&character).fetch_one(&h.pool).await?;
            let other_account=operation(200);
            source.evidence.refresh_account(&h.root,&h.node,&other_account).await.map_err(debug)?;
            let foreign=intent("rename",13,revision+1,now,&other_account,&character,None)?;
            rejected(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter",&operation(13)),Input::Intent(&foreign)).await,"CHAR_CMD_OWNER_MISMATCH")?;
            let create=create_intent(201,now,&other_account,&world,&original_name)?;
            rejected(h.root.decide_character_command(&authority,&h.node,"oteryn:character-authority:primary","CreateCharacter",&operation(201),Input::Intent(&create)).await,"CHAR_CMD_NAME_UNAVAILABLE")?;
            let collision=std::str::from_utf8(&create_intent(203,now,&other_account,&world,&original_name)?)?.replace(&operation(203),&operation(1)).into_bytes();
            assert_eq!(h.root.decide_character_command(&authority,&h.node,"oteryn:character-authority:primary","CreateCharacter",&operation(1),Input::Intent(&collision)).await,Decision::Conflict);
            let collision=std::str::from_utf8(&intent("rename",12,revision+1,now,&account,&character,None)?)?.replace(&operation(12),&operation(201)).into_bytes();
            assert_eq!(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter",&operation(201)),Input::Intent(&collision)).await,Decision::Conflict);
            let noop = intent("rename",2,revision+1,now,&account,&character,None)?;
            rejected(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter",&operation(2)),Input::Intent(&noop)).await,"CHAR_CMD_NAME_UNAVAILABLE")?;
            let second = std::str::from_utf8(&intent("rename",3,revision+1,now,&account,&character,None)?)?.replace("Aldric Vale","Aldric Moor").into_bytes();
            committed(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter",&operation(3)),Input::Intent(&second)).await)?;
            let lcfa_before: String = sqlx::query_scalar("SELECT projection_revision::text FROM game_character_account_projections WHERE account_id=$1::uuid").bind(&account).fetch_one(&h.pool).await?;
            let service_before: i64 = sqlx::query_scalar("SELECT revision FROM game_character_service_revisions WHERE account_id=$1::uuid").bind(&account).fetch_one(&h.pool).await?;
            for (kind,command,n) in [("sex","ChangeCharacterSex",4),("world","TransferCharacterWorld",5)] {
                let raw = intent(kind,n,revision+2,now,&account,&character,None)?;
                rejected(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",(command,&operation(n)),Input::Intent(&raw)).await,"CHAR_CMD_UNSUPPORTED")?;
            }
            let admission=super::fresh_fixture::fresh::Source::new(now)?;
            let mut runtime=super::fresh_fixture::fresh::Source::new(now)?;
            runtime.rows.retain(|r|matches!(r.key,crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(_)));
            h.root.publish_runtime_readiness(&h.node,&crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1::prepare(&runtime,now).map_err(debug)?).await.map_err(debug)?;
            let mut claims=super::fresh_fixture::fresh::Source::new(now)?;
            claims.rows.retain(|r|!matches!(r.key,crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(_)));
            crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(h.root.clone()).publish(&crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1::prepare(&claims,now).map_err(debug)?).await.map_err(debug)?;
            if projection_only {
                stage.set("real clock: slow root update, resync and epoch raise");
                for n in 180_u8..197 {
                    let owner=operation(n+40);
                    source.evidence.refresh_account(&h.root,&h.node,&owner).await.map_err(debug)?;
                    let name=format!("Resync {}",char::from(b'A'+(n-180)));
                    let create=create_intent(n,now,&owner,&world,&name)?;
                    committed(h.root.decide_character_command(&authority,&h.node,"oteryn:character-authority:primary","CreateCharacter",&operation(n),Input::Intent(&create)).await)?;
                }
                assert!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_roots").fetch_one(&h.pool).await?>16);
                let mut tx=h.pool.begin().await?;
                sqlx::query("CREATE OR REPLACE FUNCTION game_character_service_projection_now() RETURNS BIGINT LANGUAGE sql VOLATILE AS 'SELECT greatest(1,floor(extract(epoch FROM clock_timestamp()))::bigint)'").execute(&mut *tx).await?;
                sqlx::query("CREATE OR REPLACE FUNCTION game_character_service_now() RETURNS BIGINT LANGUAGE sql VOLATILE AS 'SELECT NULL::bigint'").execute(&mut *tx).await?;
                // A real gameplay Bestiary receipt advances root and progression together.
                sqlx::query("WITH receipt AS (INSERT INTO game_character_bestiary_kill_receipts(bestiary_occurrence_id,command_binding,race_digest,character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,race_key,race_definition_revision,final_kill_threshold,kill_count_before,kill_count_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT game_character_uuid_v7(),decode('01','hex'),sha256('slow-root'),character_id,character_revision,character_revision+1,level,level,total_experience,total_experience,'slow-root-race','definition-r1',1,0,1,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,0 FROM game_character_progression_state WHERE character_id=$1::uuid RETURNING *) INSERT INTO game_character_bestiary_progress SELECT character_id,race_key,kill_count_after,committed_character_revision,bestiary_occurrence_id FROM receipt").bind(&character).execute(&mut *tx).await?;
                sqlx::query("DO $$ BEGIN PERFORM pg_sleep(1.1); UPDATE game_character_roots SET character_revision=character_revision+1 WHERE character_id IN(SELECT character_id FROM game_character_bestiary_progress WHERE race_key='slow-root-race'); END $$").execute(&mut *tx).await?;
                sqlx::query("UPDATE game_character_progression_state SET character_revision=character_revision+1 WHERE character_id=$1::uuid").bind(&character).execute(&mut *tx).await?;
                sqlx::query("CREATE OR REPLACE FUNCTION game_character_service_now() RETURNS BIGINT LANGUAGE sql VOLATILE SET search_path=pg_catalog,public AS 'SELECT value FROM service_test_clock'").execute(&mut *tx).await?;
                tx.commit().await?;
                revision+=1;
                let mut tx=h.pool.begin().await?;
                // Reproduce the rejected one-second window using real server clocks;
                // the resync statement below must remain independent of this clock.
                sqlx::query("CREATE OR REPLACE FUNCTION game_character_service_now() RETURNS BIGINT LANGUAGE sql VOLATILE AS 'SELECT CASE WHEN abs(extract(epoch FROM clock_timestamp()-statement_timestamp()))<=1 THEN floor(extract(epoch FROM clock_timestamp()))::bigint END'").execute(&mut *tx).await?;
                let before:i64=sqlx::query_scalar("SELECT revised_at FROM game_character_service_revisions WHERE account_id=$1::uuid").bind(&account).fetch_one(&mut *tx).await?;
                sqlx::query("UPDATE game_character_service_revisions SET revised_at=revised_at+3600 WHERE account_id=$1::uuid").bind(&account).execute(&mut *tx).await?;
                sqlx::query("DO $$ BEGIN PERFORM pg_sleep(1.1); IF game_character_service_now() IS NOT NULL THEN RAISE EXCEPTION 'real-clock window did not expire'; END IF; PERFORM game_character_account_projection_resync(true); FOR i IN 1..4 LOOP PERFORM game_character_service_resync_page(); END LOOP; END $$").execute(&mut *tx).await?;
                assert_eq!(sqlx::query_scalar::<_,i64>("SELECT revised_at FROM game_character_service_revisions WHERE account_id=$1::uuid").bind(&account).fetch_one(&mut *tx).await?,before+3600);
                tx.rollback().await?;
                // Return to the scenario's injected clock after the real-clock proof.
                sqlx::query("CREATE OR REPLACE FUNCTION game_character_service_projection_now() RETURNS BIGINT LANGUAGE sql VOLATILE SET search_path=pg_catalog,public AS 'SELECT value FROM service_test_clock'").execute(&h.pool).await?;
                sqlx::query("UPDATE game_character_service_revisions SET revised_at=$1").bind(now).execute(&h.pool).await?;
            }
            stage.set("schedule and offline cancel");
            let raw = intent("schedule",6,revision+2,now,&account,&character,None)?;
            committed(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("ScheduleCharacterDeletion",&operation(6)),Input::Intent(&raw)).await)?;
            assert!(matches!(crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone()).commit(&admission.request()?).await.map_err(debug)?,crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority));
            let pending: i16 = sqlx::query_scalar("SELECT lifecycle FROM game_character_roots WHERE character_id=$1::uuid").bind(&character).fetch_one(&h.pool).await?; assert_eq!(pending,3);
            let raw = intent("schedule",7,revision+3,now,&account,&character,None)?;
            rejected(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("ScheduleCharacterDeletion",&operation(7)),Input::Intent(&raw)).await,"CHAR_CMD_LIFECYCLE_CONFLICT")?;
            let lcfa_pending: String = sqlx::query_scalar("SELECT projection_revision::text FROM game_character_account_projections WHERE account_id=$1::uuid").bind(&account).fetch_one(&h.pool).await?;
            assert_ne!(lcfa_before,lcfa_pending);
            // Updating only the deadline is a services-only mutation and cannot advance LCFA.
            sqlx::query("UPDATE game_character_service_deletions SET effective_at=effective_at+1 WHERE character_id=$1::uuid").bind(&character).execute(&h.pool).await?;
            assert_eq!(sqlx::query_scalar::<_,String>("SELECT projection_revision::text FROM game_character_account_projections WHERE account_id=$1::uuid").bind(&account).fetch_one(&h.pool).await?,lcfa_pending);
            assert!(sqlx::query_scalar::<_,i64>("SELECT revision FROM game_character_service_revisions WHERE account_id=$1::uuid").bind(&account).fetch_one(&h.pool).await?>service_before);
            let services_before_lcfa_ack:i64=sqlx::query_scalar("SELECT projection_revision-delivered_revision FROM game_character_service_outbox WHERE account_id=$1::uuid AND character_id=$2::uuid").bind(&account).bind(&character).fetch_one(&h.pool).await?;
            assert!(services_before_lcfa_ack>0);
            let lcfa=h.root.next_account_characters_snapshot(&authority).await.map_err(debug)?.ok_or("LCFA snapshot")?;
            h.root.clear_account_characters(&authority,&lcfa.account_id,lcfa.projection_epoch,lcfa.projection_revision).await.map_err(debug)?;
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT projection_revision-delivered_revision FROM game_character_service_outbox WHERE account_id=$1::uuid AND character_id=$2::uuid").bind(&account).bind(&character).fetch_one(&h.pool).await?,services_before_lcfa_ack);
            if projection_only {return Ok(());}
            let raw = intent("cancel",8,revision+3,now,&account,&character,Some(6))?;
            sqlx::query("CREATE OR REPLACE FUNCTION game_character_service_now() RETURNS BIGINT LANGUAGE sql VOLATILE AS 'SELECT NULL::bigint'").execute(&h.pool).await?;
            assert_eq!(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("CancelCharacterDeletion",&operation(8)),Input::Intent(&raw)).await,Decision::Transient);
            assert!(!h.root.finalize_character_service_deletion(&authority,&h.node).await.map_err(debug)?);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_service_receipts WHERE operation_id=$1::uuid").bind(operation(8)).fetch_one(&h.pool).await?,0);
            sqlx::query("CREATE OR REPLACE FUNCTION game_character_service_now() RETURNS BIGINT LANGUAGE sql VOLATILE SET search_path=pg_catalog,public AS 'SELECT value FROM service_test_clock'").execute(&h.pool).await?;
            committed(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("CancelCharacterDeletion",&operation(8)),Input::Intent(&raw)).await)?;
            let raw = intent("schedule",9,revision+4,now,&account,&character,None)?;
            committed(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("ScheduleCharacterDeletion",&operation(9)),Input::Intent(&raw)).await)?;
            let deadline: i64 = sqlx::query_scalar("SELECT effective_at FROM game_character_service_deletions WHERE character_id=$1::uuid").bind(&character).fetch_one(&h.pool).await?;
            assert_eq!(deadline-now,60*86400);
            // Expiry/unsafe clock cannot shorten the deletion policy. At the boundary,
            // finalization uses the durable deadline even without a new Platform command.
            sqlx::query("UPDATE service_test_clock SET value=$1").bind(deadline).execute(&h.pool).await?;
            sqlx::query("UPDATE preadm_clock.instant SET value=$1").bind(deadline).execute(&h.pool).await?;
            super::HOST_NOW.replace(Some(deadline));
            source.clock.store(deadline as u64,std::sync::atomic::Ordering::SeqCst);
            source.evidence.refresh_account(&h.root,&h.node,&account).await.map_err(debug)?;
            let cancel_at_deadline=intent("cancel",10,revision+5,deadline,&account,&character,Some(9))?;
            stage.set("finalization and name retention boundary");
            if race {
                let other=h.independent_root()?;
                assert!(other.maintain_ready_once().await?);
                let other_authority=other.open_character_authority(&seal).await.map_err(debug)?;
                let cancel_operation=operation(10);
                let (cancel,finalize)=tokio::join!(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("CancelCharacterDeletion",&cancel_operation),Input::Intent(&cancel_at_deadline)),other.finalize_character_service_deletion(&other_authority,&h.node));
                rejected(cancel,"CHAR_CMD_DELETION_WINDOW_CLOSED")?;
                assert!(finalize.map_err(debug)?);
            } else {
                rejected(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("CancelCharacterDeletion",&operation(10)),Input::Intent(&cancel_at_deadline)).await,"CHAR_CMD_DELETION_WINDOW_CLOSED")?;
                assert!(h.root.finalize_character_service_deletion(&authority,&h.node).await.map_err(debug)?);
            }
            assert!(!h.root.finalize_character_service_deletion(&authority,&h.node).await.map_err(debug)?);
            assert_eq!(sqlx::query_scalar::<_,i16>("SELECT lifecycle FROM game_character_roots WHERE character_id=$1::uuid").bind(&character).fetch_one(&h.pool).await?,4);
            sqlx::query("UPDATE service_test_clock SET value=$1").bind(deadline+30*86400).execute(&h.pool).await?;
            sqlx::query("SELECT game_character_service_release_name($1)").bind(&held.0).execute(&h.pool).await?;
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_name_reservations WHERE name_key=$1").bind(&held.0).fetch_one(&h.pool).await?,0);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_roots WHERE character_id=$1::uuid").bind(&character).fetch_one(&h.pool).await?,1);
            let reuse_time=deadline+30*86400;
            sqlx::query("UPDATE preadm_clock.instant SET value=$1").bind(reuse_time).execute(&h.pool).await?;
            super::HOST_NOW.replace(Some(reuse_time)); source.clock.store(reuse_time as u64,std::sync::atomic::Ordering::SeqCst);
            source.evidence.refresh_account(&h.root,&h.node,&other_account).await.map_err(debug)?;
            let create=create_intent(202,reuse_time,&other_account,&world,&original_name)?;
            committed(h.root.decide_character_command(&authority,&h.node,"oteryn:character-authority:primary","CreateCharacter",&operation(202),Input::Intent(&create)).await)?;

            stage.set("shared-epoch resync retains tombstones and stale ACK cannot clear");
            let epoch:i64=sqlx::query_scalar("SELECT game_character_account_projection_resync(true)").fetch_one(&h.pool).await?;
            assert!(epoch>snapshot.epoch as i64);
            for _ in 0..8 {
                let _=delivery.next().await.map_err(debug)?;
                if sqlx::query_scalar::<_,i16>("SELECT phase FROM game_character_service_feed WHERE singleton").fetch_one(&h.pool).await?==3 {break;}
            }
            assert_eq!(sqlx::query_scalar::<_,i16>("SELECT phase FROM game_character_service_feed WHERE singleton").fetch_one(&h.pool).await?,3);
            let tombstone:Value=sqlx::query_scalar("SELECT snapshot FROM game_character_service_outbox WHERE account_id=$1::uuid AND character_id=$2::uuid").bind(&account).bind(&character).fetch_one(&h.pool).await?;
            assert_eq!(tombstone["lifecycle"],"DELETED");assert_eq!(tombstone["deletion_operation_id"],json!(operation(9)));assert_eq!(tombstone["projection_epoch"],json!(epoch.to_string()));
            delivery.acknowledge(&snapshot).await.map_err(debug)?;
            assert!(sqlx::query_scalar::<_,bool>("SELECT projection_revision>delivered_revision FROM game_character_service_outbox WHERE account_id=$1::uuid AND character_id=$2::uuid").bind(&account).bind(&character).fetch_one(&h.pool).await?);
            stage.set("durable projection quarantine cannot be cleared by an ACK");
            let current=delivery.next().await.map_err(debug)?.ok_or("resynced service snapshot")?;
            delivery.acknowledge(&current).await.map_err(debug)?;
            delivery.quarantine(&current).await.map_err(debug)?;
            delivery.acknowledge(&current).await.map_err(debug)?;
            assert!(sqlx::query_scalar::<_,bool>("SELECT quarantined AND projection_revision>delivered_revision FROM game_character_service_outbox WHERE account_id=$1::uuid AND character_id=$2::uuid").bind(&current.account_id).bind(&current.character_id).fetch_one(&h.pool).await?);
            assert!(delivery.facts().await.map_err(debug)?.oldest_undelivered_ms.is_some());
            let candidate=delivery.next().await.map_err(debug)?;
            assert!(!candidate.is_some_and(|row|row.account_id==current.account_id));
            stage.set("durable refusal survives restart");
            h.root.stop_character_service_receipt(&authority,&h.node,&operation(1),&receipt).await.map_err(debug)?;
            let restarted=h.independent_root()?;
            assert!(restarted.maintain_ready_once().await?);
            let restarted_authority=restarted.open_character_authority(&seal).await.map_err(debug)?;
            assert_eq!(restarted.decide_character_service(&restarted_authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter",&operation(1)),Input::Intent(&original_intent)).await,Decision::Conflict);
            Ok(())
        }.await;
            stage.set("cleanup");
            let cleanup = h.cleanup().await;
            result?;
            cleanup
        })
    })
}
fn committed(decision: Decision) -> TestResult<Vec<u8>> {
    match decision {
        Decision::Publish {
            receipt,
            result_code,
        } if result_code == "CHAR_CMD_OK" => Ok(receipt),
        other => Err(format!("expected committed service, got {other:?}").into()),
    }
}
fn rejected(decision: Decision, expected: &str) -> TestResult {
    match decision {
        Decision::Publish {
            receipt,
            result_code,
        } => {
            assert_eq!(result_code, expected);
            let body: Value = serde_json::from_slice(&receipt)?;
            assert_eq!(body["outcome"], "REJECTED");
            assert!(body.get("character_id").is_none());
            Ok(())
        }
        other => Err(format!("expected service refusal, got {other:?}").into()),
    }
}
fn operation(n: u8) -> String {
    format!("0192a000-0000-7000-8000-{n:012x}")
}
fn intent(
    kind: &str,
    n: u8,
    revision: u64,
    now: i64,
    account: &str,
    character: &str,
    schedule: Option<u8>,
) -> TestResult<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/character-services-v2")
        .join(format!("{kind}.json"));
    let mut value: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    value["operation_id"] = json!(operation(n));
    value["issuer_decision_id"] = json!(operation(n + 100));
    value["source_revision"] = json!(n.to_string());
    value["character_id"] = json!(character);
    value["account_id"] = json!(account);
    value["expected_character_revision"] = json!(revision.to_string());
    value["issued_at_source"] = json!(now.to_string());
    value["expires_at_source"] = json!((now + 300).to_string());
    if let Some(schedule) = schedule {
        value["deletion_operation_id"] = json!(operation(schedule));
    }
    let fields = [
        "contract_version",
        "command",
        "variant",
        "issuer_authority",
        "issuer_decision_id",
        "source_revision",
        "operation_id",
        "character_id",
        "account_id",
        "expected_character_revision",
        "deletion_operation_id",
        "requested_name",
        "requested_sex",
        "expected_from_world_id",
        "target_world_id",
        "issued_at_source",
        "expires_at_source",
        "audience",
    ]
    .into_iter()
    .filter_map(|key| value.get(key).map(|v| (key.to_owned(), v.clone())))
    .collect();
    CharacterCommandIntentV2::from_fields(fields)
        .and_then(|v| v.encode())
        .map_err(debug)
}

fn create_intent(n: u8, now: i64, account: &str, world: &str, name: &str) -> TestResult<Vec<u8>> {
    #[derive(serde::Serialize)]
    struct Create<'a> {
        contract_version: u8,
        command: &'a str,
        variant: &'a str,
        issuer_authority: &'a str,
        issuer_decision_id: String,
        source_revision: String,
        operation_id: String,
        account_id: &'a str,
        target_world_id: &'a str,
        requested_name: &'a str,
        issued_at_source: String,
        expires_at_source: String,
        audience: &'a str,
    }
    Ok(serde_json::to_vec(&Create {
        contract_version: 1,
        command: "CreateCharacter",
        variant: "PLATFORM_USER_CREATE",
        issuer_authority: "OTERYN_PLATFORM_CHARACTER_AUTHORITY",
        issuer_decision_id: operation(n - 50),
        source_revision: n.to_string(),
        operation_id: operation(n),
        account_id: account,
        target_world_id: world,
        requested_name: name,
        issued_at_source: now.to_string(),
        expires_at_source: (now + 300).to_string(),
        audience: "OTERYN_GAME_CHARACTER_AUTHORITY",
    })?)
}
