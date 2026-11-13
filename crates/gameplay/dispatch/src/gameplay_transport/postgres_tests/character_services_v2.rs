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
#[test]
fn character_services_v2_review_oversize_is_terminal() -> TestResult {
    scenario(false,"char_services_review_oversize",false)
}
#[test]
fn character_services_v2_review_resync_uses_current_owner() -> TestResult {
    scenario(false,"char_services_review_owner",false)
}
#[test]
fn character_services_v2_review_same_epoch_quarantine_survives() -> TestResult {
    scenario(false,"char_services_review_quarantine",false)
}
#[test]
fn character_services_v2_review_migration_has_no_root_index_build() -> TestResult {
    scenario(false,"char_services_review_migration",false)
}
#[test]
fn character_services_v2_review_receipt_replays_with_security_peer_down() -> TestResult {
    scenario(false,"char_services_review_replay",false)
}
#[test]
fn character_services_v2_review_finalizer_skips_busy_oldest() -> TestResult {
    scenario(false,"char_services_review_finalizer",false)
}
#[test]
fn character_services_v2_review_tombstones_do_not_occupy_portfolio() -> TestResult {
    scenario(false,"char_services_review_portfolio",false)
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

            if tag.ends_with("migration") {
                assert!(!sqlx::query_scalar::<_,bool>("SELECT to_regclass('game_character_service_resync_roots') IS NOT NULL").fetch_one(&h.pool).await?,"resync must reuse the existing root primary key");
                assert!(sqlx::query_scalar::<_,bool>("SELECT convalidated FROM pg_constraint WHERE conrelid='game_character_roots'::regclass AND conname='game_character_roots_lifecycle_check'").fetch_one(&h.pool).await?);
                let mut ddl=h.pool.begin().await?;
                sqlx::query("ALTER TABLE game_character_roots DROP CONSTRAINT game_character_roots_lifecycle_check").execute(&mut *ddl).await?;
                sqlx::query("ALTER TABLE game_character_roots ADD CONSTRAINT game_character_roots_lifecycle_check CHECK(lifecycle IN(1,2,3,4)) NOT VALID").execute(&mut *ddl).await?;
                ddl.commit().await?;
                let mut writer=h.pool.begin().await?;
                sqlx::query("LOCK TABLE game_character_roots IN ROW EXCLUSIVE MODE").execute(&mut *writer).await?;
                let mut validator=h.pool.begin().await?;
                sqlx::query("SET LOCAL lock_timeout='1s'").execute(&mut *validator).await?;
                sqlx::query("ALTER TABLE game_character_roots VALIDATE CONSTRAINT game_character_roots_lifecycle_check").execute(&mut *validator).await?;
                validator.commit().await?;writer.rollback().await?;
                return Ok(());
            }
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
            if tag.ends_with("oversize") {
                let oversized=vec![b' ';4097];
                let first=h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter",&operation(20)),Input::Intent(&oversized)).await;
                rejected(first.clone(),"CHAR_CMD_INTENT_INVALID")?;
                assert_eq!(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter",&operation(20)),Input::Oversized).await,first);
                assert_eq!(sqlx::query_scalar::<_,String>("SELECT character_revision::text FROM game_character_roots WHERE character_id=$1::uuid").bind(&character).fetch_one(&h.pool).await?,revision.to_string());
                return Ok(());
            }
            let raw = intent("rename",1,revision,now,&account,&character,None)?;
            let original_intent=raw.clone();
            stage.set("rename and immutable replay");
            let first = h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter", &operation(1)),Input::Intent(&raw)).await;
            let receipt = committed(first)?;
            assert_eq!(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter",&operation(1)),Input::Intent(&raw)).await,Decision::Publish { receipt: receipt.clone(),result_code:"CHAR_CMD_OK".into() });
            if tag.ends_with("replay") {
                source.stop_peer();tokio::task::yield_now().await;
                assert!(source.evidence.refresh_account(&h.root,&h.node,&account).await.is_err());
                assert_eq!(h.root.replay_character_service(&authority,&h.node,"RenameCharacter",&operation(1),Input::Intent(&raw)).await.map_err(debug)?,Some(Decision::Publish{receipt:receipt.clone(),result_code:"CHAR_CMD_OK".into()}));
                let changed=std::str::from_utf8(&raw)?.replace("Aldric Vale","Aldric View").into_bytes();
                assert_eq!(h.root.replay_character_service(&authority,&h.node,"RenameCharacter",&operation(1),Input::Intent(&changed)).await.map_err(debug)?,Some(Decision::Conflict));
                return Ok(());
            }
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
            if tag.ends_with("owner") {
                stage.set("ownership returns while resync waits for portfolio serialization");
                assert!(!sqlx::query_scalar::<_,bool>("SELECT has_table_privilege('oteryn_game_runtime','game_bazaar_configuration','UPDATE')").fetch_one(&h.pool).await?);
                let other=operation(220);
                source.evidence.refresh_account(&h.root,&h.node,&other).await.map_err(debug)?;
                let out=transfer_intent(30,now,&character,&account,&other);
                committed(h.root.decide_character_command(&authority,&h.node,"oteryn:character-authority:primary","TransferCharacterOwnership",&operation(30),Input::Intent(&out)).await)?;
                sqlx::query("UPDATE game_character_service_feed SET phase=2,after_account='00000000-0000-0000-0000-000000000000',after_character='00000000-0000-0000-0000-000000000000' WHERE singleton").execute(&h.pool).await?;
                let mut held=h.pool.begin().await?;
                sqlx::query("SELECT 1 FROM game_character_account_guards WHERE account_id=$1::uuid FOR UPDATE").bind(&account).execute(&mut *held).await?;
                let back=transfer_intent(31,now,&character,&other,&account);
                let back_operation=operation(31);
                let transfer=h.root.decide_character_command(&authority,&h.node,"oteryn:character-authority:primary","TransferCharacterOwnership",&back_operation,Input::Intent(&back));
                let coordinate=async {
                    wait_for_review_lock(&h.pool,"game_character_account_guards").await?;
                    let mut resync=h.pool.begin().await?;
                    sqlx::query("SET LOCAL application_name='charv2-resync-race'").execute(&mut *resync).await?;
                    let run=async {
                        sqlx::query("SELECT game_character_service_resync_page()").execute(&mut *resync).await?;
                        resync.commit().await?;
                        Ok::<(),Box<dyn std::error::Error>>(())
                    };
                    let release=async {
                        wait_for_review_lock(&h.pool,"game_character_service_resync_page").await?;
                        held.commit().await?;
                        Ok::<(),Box<dyn std::error::Error>>(())
                    };
                    let (run,release)=tokio::join!(run,release);run?;release?;
                    Ok::<(),Box<dyn std::error::Error>>(())
                };
                let (returned,coordinated)=tokio::join!(transfer,coordinate);committed(returned)?;coordinated?;
                let facts:Value=sqlx::query_scalar("SELECT snapshot FROM game_character_service_outbox WHERE account_id=$1::uuid AND character_id=$2::uuid").bind(&account).bind(&character).fetch_one(&h.pool).await?;
                assert_eq!(facts["lifecycle"],"ACTIVE");
                assert_eq!(facts["account_id"],json!(account));
                return Ok(());
            }
            if tag.ends_with("quarantine") {
                stage.set("retained removal is not current ownership; quarantine survives resync");
                let mut tx=h.pool.begin().await?;
                sqlx::query("UPDATE game_character_service_feed SET phase=2,after_account='00000000-0000-0000-0000-000000000000',after_character='00000000-0000-0000-0000-000000000000' WHERE singleton").execute(&mut *tx).await?;
                sqlx::query("UPDATE game_character_service_outbox SET snapshot=jsonb_set(snapshot,'{lifecycle}','\"REMOVED_FROM_ACCOUNT\"'),quarantined=$3,delivered_revision=projection_revision WHERE account_id=$1::uuid AND character_id=$2::uuid").bind(&account).bind(&character).bind(tag.ends_with("quarantine")).execute(&mut *tx).await?;
                sqlx::query("SELECT game_character_service_resync_page()").execute(&mut *tx).await?;
                let (life,quarantined):(String,bool)=sqlx::query_as("SELECT snapshot->>'lifecycle',quarantined FROM game_character_service_outbox WHERE account_id=$1::uuid AND character_id=$2::uuid").bind(&account).bind(&character).fetch_one(&mut *tx).await?;
                if tag.ends_with("owner") {assert_eq!(life,"ACTIVE");} else {assert!(quarantined,"same-epoch resync must not repair quarantine");}
                tx.rollback().await?;
                return Ok(());
            }
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
            if tag.ends_with("finalizer") {
                // Seed the retained terminal session before scheduling, under the real admission guard.
                sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,1,$4::uuid,encode($5,'hex')::uuid,1,999999,1,1,1,1,$6,3)").bind(operation(24)).bind(&account).bind(&character).bind(&world).bind(id(43).as_slice()).bind([29_u8;16].as_slice()).execute(&h.pool).await?;
            }
            stage.set("schedule and offline cancel");
            let raw = intent("schedule",6,revision+2,now,&account,&character,None)?;
            committed(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("ScheduleCharacterDeletion",&operation(6)),Input::Intent(&raw)).await)?;
            assert!(matches!(crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone()).commit(&admission.request()?).await.map_err(debug)?,crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority));
            if tag.ends_with("finalizer") {
                stage.set("busy oldest deletion cannot block a later offline deadline");
                let create=create_intent(140,now,&account,&world,"Finalizer Later")?;
                let created=committed(h.root.decide_character_command(&authority,&h.node,"oteryn:character-authority:primary","CreateCharacter",&operation(140),Input::Intent(&create)).await)?;
                let created:Value=serde_json::from_slice(&created)?;let second=created["character_id"].as_str().ok_or("second root")?;
                sqlx::query("INSERT INTO game_character_progression_state SELECT $1::uuid,1,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=$2::uuid").bind(second).bind(&character).execute(&h.pool).await?;
                sqlx::query("UPDATE service_test_clock SET value=value+1").execute(&h.pool).await?;
                let schedule=intent("schedule",22,1,now+1,&account,second,None)?;
                committed(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("ScheduleCharacterDeletion",&operation(22)),Input::Intent(&schedule)).await)?;
                // A retained terminal session can still have a presence holder awaiting release.
                sqlx::query("UPDATE game_durability_admission_account_guards SET presence_character_id=$2::uuid,holder_game_session_id=$3::uuid WHERE account_id=$1::uuid").bind(&account).bind(&character).bind(operation(24)).execute(&h.pool).await?;
                let deadline:i64=sqlx::query_scalar("SELECT max(effective_at) FROM game_character_service_deletions").fetch_one(&h.pool).await?;
                sqlx::query("UPDATE service_test_clock SET value=$1").bind(deadline).execute(&h.pool).await?;
                sqlx::query("UPDATE preadm_clock.instant SET value=$1").bind(deadline).execute(&h.pool).await?;
                super::HOST_NOW.replace(Some(deadline));
                assert!(h.root.finalize_character_service_deletion(&authority,&h.node).await.map_err(debug)?);
                assert_eq!(sqlx::query_scalar::<_,i16>("SELECT lifecycle FROM game_character_roots WHERE character_id=$1::uuid").bind(second).fetch_one(&h.pool).await?,4);
                assert_eq!(sqlx::query_scalar::<_,i16>("SELECT lifecycle FROM game_character_roots WHERE character_id=$1::uuid").bind(&character).fetch_one(&h.pool).await?,3);
                return Ok(());
            }
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

            if tag.ends_with("portfolio") {
                stage.set("64 live roots plus retained tombstone can enter account closure");
                source.evidence.refresh_account(&h.root,&h.node,&account).await.map_err(debug)?;
                for n in 70_u8..134 {
                    let name=format!("Portfolio {}{}",char::from(b'A'+(n-70)/26),char::from(b'A'+(n-70)%26));
                    let create=create_intent(n,reuse_time,&account,&world,&name)?;
                    committed(h.root.decide_character_command(&authority,&h.node,"oteryn:character-authority:primary","CreateCharacter",&operation(n),Input::Intent(&create)).await)?;
                }
                assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_roots WHERE account_id=$1::uuid").bind(&account).fetch_one(&h.pool).await?,65);
                assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_live_roots WHERE account_id=$1::uuid").bind(&account).fetch_one(&h.pool).await?,64);
                use crate::durability::character_command::account_deletion::{Config,Intent};
                use crate::native_admission_source::account_closure::{LifecycleRequest,LifecycleObservation};
                let close=Intent{contract_version:2,command:"CloseAccount".into(),variant:"PLATFORM_ACCOUNT_CLOSE".into(),issuer_authority:"OTERYN_PLATFORM_CHARACTER_AUTHORITY".into(),issuer_decision_id:operation(230),source_revision:"230".into(),operation_id:operation(231),account_id:account.clone(),termination_id:operation(232),lifecycle_generation:"8".into(),expected_recovery_generation:"1".into(),close_operation_id:None,decision:None,scheduled_for:(reuse_time-1).to_string(),issued_at_source:(reuse_time-1).to_string(),expires_at_source:(reuse_time+120).to_string(),audience:"OTERYN_GAME_CHARACTER_AUTHORITY".into()};
                let raw=serde_json::to_vec(&close)?;
                let request=LifecycleRequest::from_intent("oteryn:character-authority:primary",&raw,close.expected_recovery_generation.parse()?).map_err(debug)?;
                let observation=LifecycleObservation{contract_version:2,operation:"ReadAccountClosureLifecycleV1".into(),source_authority:"OTERYN_PLATFORM_CHARACTER_AUTHORITY".into(),account_id:account.clone(),termination_id:close.termination_id.clone(),lifecycle_generation:"8".into(),source_revision:"1".into(),phase:"CLOSE".into(),financial_obligations_clear:true,recovery_generation:close.expected_recovery_generation.clone(),observed_at:reuse_time.to_string(),valid_until:(reuse_time+5).to_string()};
                sqlx::query("CREATE OR REPLACE FUNCTION game_account_closure_now() RETURNS BIGINT LANGUAGE sql VOLATILE SET search_path=pg_catalog,public AS 'SELECT value FROM service_test_clock'").execute(&h.pool).await?;
                h.root.observe_account_closure_lifecycle(&authority,&h.node,&request,&serde_json::to_vec(&observation)?).await.map_err(debug)?;
                let _=h.root.decide_character_command_configured(&authority,&h.node,"oteryn:character-authority:primary",("CloseAccount",&close.operation_id),Input::Intent(&raw),&Config::default()).await;
                assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_account_closure_members WHERE close_operation_id=$1::uuid").bind(&close.operation_id).fetch_one(&h.pool).await?,64);
                assert!(!sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_account_closure_members WHERE character_id=$1::uuid)").bind(&character).fetch_one(&h.pool).await?);
                return Ok(());
            }
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

fn transfer_intent(n:u8,now:i64,character:&str,from:&str,to:&str)->Vec<u8> {
    format!(r#"{{"contract_version":1,"command":"TransferCharacterOwnership","variant":"PLATFORM_BAZAAR_TRANSFER","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"{}","source_revision":"{}","operation_id":"{}","character_id":"{}","expected_from_account_id":"{}","to_account_id":"{}","purpose":"BAZAAR_LISTING_ESCROW","issued_at_source":"{}","expires_at_source":"{}","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}}"#,operation(n+100),n,operation(n),character,from,to,now,now+300).into_bytes()
}
async fn wait_for_review_lock(pool:&sqlx::PgPool,query:&str)->TestResult {
    loop {
        if sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND pid<>pg_backend_pid() AND wait_event_type='Lock' AND query LIKE '%'||$1||'%')").bind(query).fetch_one(pool).await? {return Ok(());}
        tokio::task::yield_now().await;
    }
}
