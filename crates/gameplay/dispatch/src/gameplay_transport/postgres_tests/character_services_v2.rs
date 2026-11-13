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
    scenario(false)
}
#[test]
fn character_services_v2_postgres_cancel_finalizer_race_has_one_winner() -> TestResult {
    scenario(true)
}
fn scenario(race: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::journal_watchdog::run("character-services-v2", move |stage| {
        run(async move |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "character_services_v2", true, false)
                    .await?;
            let result: TestResult = async {
            stage.set("fixed clock and authority");
            let now: i64 = sqlx::query_scalar("SELECT game_character_service_now()").fetch_one(&h.pool).await?;
            sqlx::query("CREATE TABLE service_test_clock(value BIGINT NOT NULL)").execute(&h.pool).await?;
            sqlx::query("INSERT INTO service_test_clock VALUES($1)").bind(now).execute(&h.pool).await?;
            sqlx::query("GRANT SELECT ON service_test_clock TO oteryn_game_runtime").execute(&h.pool).await?;
            sqlx::query("CREATE OR REPLACE FUNCTION game_character_service_now() RETURNS BIGINT LANGUAGE sql VOLATILE SET search_path=pg_catalog,public AS 'SELECT value FROM service_test_clock'").execute(&h.pool).await?;
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
            let revision: u64 = root.try_get::<String,_>("character_revision")?.parse()?;
            let raw = intent("rename",1,revision,now,&account,&character,None)?;
            stage.set("rename and immutable replay");
            let first = h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter", &operation(1)),Input::Intent(&raw)).await;
            let receipt = committed(first)?;
            assert_eq!(h.root.decide_character_service(&authority,&h.node,"oteryn:character-authority:primary",("RenameCharacter",&operation(1)),Input::Intent(&raw)).await,Decision::Publish { receipt: receipt.clone(),result_code:"CHAR_CMD_OK".into() });
            let held: (String,i64,i64) = sqlx::query_as("SELECT name_key,released_at,available_at FROM game_character_service_name_holds WHERE character_id=$1::uuid").bind(&character).fetch_one(&h.pool).await?;
            assert_eq!(held.0,original_name.to_ascii_lowercase().replace(' ',"")); assert_eq!(held.2-held.1,30*86400);
            sqlx::query("SELECT game_character_service_release_name($1)").bind(&held.0).execute(&h.pool).await?;
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_name_reservations WHERE name_key=$1").bind(&held.0).fetch_one(&h.pool).await?,1);
            let world: String=sqlx::query_scalar("SELECT world_id::text FROM game_character_roots WHERE character_id=$1::uuid").bind(&character).fetch_one(&h.pool).await?;
            let other_account=operation(200);
            source.evidence.refresh_account(&h.root,&h.node,&other_account).await.map_err(debug)?;
            let create=create_intent(201,now,&other_account,&world,&original_name)?;
            rejected(h.root.decide_character_command(&authority,&h.node,"oteryn:character-authority:primary","CreateCharacter",&operation(201),Input::Intent(&create)).await,"CHAR_CMD_NAME_UNAVAILABLE")?;
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
            let raw = intent("cancel",8,revision+3,now,&account,&character,Some(6))?;
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
