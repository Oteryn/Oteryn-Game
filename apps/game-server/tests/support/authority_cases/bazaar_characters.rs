//! Complete Character custody saga through the production Game authority.
use super::*;
use durability::bazaar::Policy;
use native_admission_source::bazaar::transport::{DeliveryKey, RemoteState};
use native_admission_source::character_commands::{Decision, Input};
use oteryn_gameplay_content::content::{
    ReferenceArtifactProjection, ReferencePlayableArtifactView,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;

const SOURCE: &str = "oteryn:character-authority:primary";
const SNAPSHOT_SOURCE: &str = "oteryn:game:primary";
const WORLD: u8 = 42;
const CHANNEL: u8 = 43;

#[test]
fn bazaar_characters_postgres_lock_bind_transfer_unlock_replay_and_fences() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64<<20).spawn(move||->Result<(),String>{
        (||->TestResult{
            tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async move{
                let started=std::time::Instant::now();
                let database=Database::create(admin,"bazaar_characters").await?;
                let result=characters_flow(&database).await;
                database.cleanup().await?;
                eprintln!("GPROD_BAZAAR_PG_EXECUTED character_lock_bind_transfer_unlock elapsed_ms={} outcome={}",started.elapsed().as_millis(),if result.is_ok(){"PASS"}else{"FAIL"});
                result
            })
        })().map_err(|error|error.to_string())
    })?.join().map_err(|_|"Bazaar Character PostgreSQL worker failed")?.map_err(Into::into)
}
fn u(tag: u8) -> String {
    uuid(id(tag))
}
async fn assert_bazaar_admission_refused(
    pool: &sqlx::PgPool,
    character: &str,
    account: u8,
    world: u8,
) -> TestResult {
    let error = sqlx::query(
        "INSERT INTO game_durability_admission_character_guards VALUES \
         ($1::uuid,$2::uuid,$3::uuid,true,1,NULL,1,'test',1,'bazaar-stale',1,0,'{}')",
    )
    .bind(character)
    .bind(u(account))
    .bind(u(world))
    .execute(pool)
    .await
    .err()
    .ok_or("Bazaar admission fence accepted an invalid source")?;
    let database = error.as_database_error().ok_or("expected SQL refusal")?;
    assert_eq!(database.code().as_deref(), Some("23514"));
    assert_eq!(database.message(), "Bazaar admission owner/lifecycle fence");
    Ok(())
}
fn hash(raw: &[u8]) -> String {
    Sha256::digest(raw)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn v2(command: &str, operation: u8, character: &str, members: &str, now: i64) -> String {
    format!(
        r#"{{"contract_version":2,"command":"{command}","variant":"PLATFORM_BAZAAR","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","issuer_decision_id":"{}","source_revision":"{operation}","operation_id":"{}","listing_id":"{}","seller_account_id":"{}","world_id":"{}","character_id":"{character}",{members},"issued_at_source":"{}","expires_at_source":"{}","audience":"OTERYN_GAME_BAZAAR_AUTHORITY"}}"#,
        u(200),
        u(operation),
        u(201),
        u(31),
        u(WORLD),
        now - 1,
        now + 120
    )
}
fn v1(operation: u8, character: &str, from: u8, to: u8, purpose: &str, now: i64) -> String {
    format!(
        r#"{{"contract_version":1,"command":"TransferCharacterOwnership","variant":"PLATFORM_BAZAAR_TRANSFER","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"{}","source_revision":"{operation}","operation_id":"{}","character_id":"{character}","expected_from_account_id":"{}","to_account_id":"{}","purpose":"{purpose}","issued_at_source":"{}","expires_at_source":"{}","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}}"#,
        u(202),
        u(operation),
        u(from),
        u(to),
        now - 1,
        now + 120
    )
}
#[track_caller]
fn receipt(decision: &Decision, code: &str) -> TestResult<Value> {
    let Decision::Publish {
        receipt,
        result_code,
    } = decision
    else {
        return Err(format!(
            "expected {code}, got {decision:?} at {}",
            std::panic::Location::caller()
        )
        .into());
    };
    assert_eq!(result_code, code);
    Ok(serde_json::from_slice(receipt)?)
}

async fn characters_flow(database: &Database) -> TestResult {
    let root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&database.url).await?;
    let directory = fence_parent().join(format!("bazaar-character-custody-{}", database.name));
    std::fs::create_dir(&directory)?;
    let recovery = CharacterRecoveryStore::open(&directory, "character-primary", "game-ops")
        .map_err(|error| format!("{error:?}"))?;
    let fresh = recovery
        .authorize_fresh_store(id(11), 100)
        .map_err(|error| format!("{error:?}"))?;
    root.admit_fresh_character_recovery(&fresh)
        .await
        .map_err(|error| format!("{error:?}"))?;
    drop(fresh);
    let fence = recovery
        .seal_current()
        .map_err(|error| format!("{error:?}"))?;
    let authority = root
        .open_character_authority(&fence)
        .await
        .map_err(|error| format!("{error:?}"))?;
    let node = register(&root, 1, None).await?;
    assign_world(&pool, &root, &node, WORLD, CHANNEL).await?;
    configure(&pool, ["profile-1", "ruleset-1", "content-1", "starter-1"]).await?;
    initialize_s2(&root, &node).await?;
    allow(&root, &node, 31).await?;
    let mut bootstrap = Wire::new(50, 31, 1)?;
    bootstrap.world = WORLD;
    let created = root
        .bootstrap_character(&authority, &node, &bootstrap.decode()?)
        .await
        .map_err(|error| format!("{error:?}"))?;
    let character = uuid(*created.character_id.as_bytes());
    // Existing typed revision-one initialization; no guard or proof is disabled.
    sqlx::query("INSERT INTO game_character_progression_state(character_id,character_revision,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES($1::uuid,1,1,0,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')")
        .bind(&character).execute(&pool).await?;
    let (artifact, _) = super::bazaar_items::artifact()?;
    let artifact_path = directory.join("server-artifact.bin");
    std::fs::write(&artifact_path, &artifact)?;
    let view = ReferencePlayableArtifactView::load(
        &artifact,
        ReferenceArtifactProjection::ServerAuthoritative,
    )?;
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,$1::uuid,$2::uuid,4)").bind(u(WORLD)).bind(u(CHANNEL)).execute(&pool).await?;
    assert!(
        root.record_content_activation(&durability::content_activation::ContentActivationRequest {
            world_id: foundation::WorldId::decode(&id(WORLD))
                .map_err(|error| format!("{error:?}"))?,
            channel_id: foundation::ChannelId::decode(&id(CHANNEL))
                .map_err(|error| format!("{error:?}"))?,
            activation_sequence: 1,
            previous_sequence: None,
            server_artifact_digest: view.artifact_digest(),
            client_artifact_digest: [3; 32],
            frame_binding_digest: [4; 32]
        })
        .await?
    );
    let key_path = directory.join("snapshot-seed");
    std::fs::write(&key_path, [23; 32])?;
    std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;
    let signing = ed25519_dalek::SigningKey::from_bytes(&[23; 32]);
    let mut keys = BTreeMap::new();
    keys.insert(
        "bazaar-pg-current".to_owned(),
        signing
            .verifying_key()
            .to_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    );
    // The authority's preloaded Content cache is intentionally private and is
    // never supplied by configuration or by qualification callers.
    let mut policy = Policy::default();
    policy.obligations_revision = Some(1);
    policy.cooldown_seconds = Some(0);
    policy.snapshot_survives_transfer = true;
    policy.escrow_account_id = Some(u(33));
    policy.escrow_capacity = Some(64);
    policy.common_restore_scope = true;
    policy.proof_retention_seconds = Some(7_776_000);
    policy.v2_receipt_bytes = Some(270336);
    policy.snapshot_source_authority = Some(SNAPSHOT_SOURCE.to_owned());
    policy.signing_key_file = Some(key_path.to_str().ok_or("key path")?.to_owned());
    policy.signing_kid = Some("bazaar-pg-current".to_owned());
    policy.signing_public_keys = keys;
    policy.content_artifact_file = Some(artifact_path.to_str().ok_or("artifact path")?.to_owned());
    policy.content_artifact_sha256 = Some(hash(&artifact));
    sqlx::query(
        "UPDATE game_bazaar_configuration SET enabled=true,policy=$1::jsonb WHERE singleton",
    )
    .bind(serde_json::to_value(&policy)?)
    .execute(&pool)
    .await?;
    let decide = async |command: &str, operation: u8, raw: &str| {
        root.decide_bazaar_command(
            &authority,
            &node,
            SOURCE,
            command,
            &u(operation),
            raw.as_bytes(),
        )
        .await
    };
    let transfer = async |operation: u8, raw: &str| {
        root.decide_character_command(
            &authority,
            &node,
            SOURCE,
            "TransferCharacterOwnership",
            &u(operation),
            Input::Intent(raw.as_bytes()),
        )
        .await
    };
    macro_rules! drain_transport {
        ($authority:expr, $proof:expr, $epoch:expr) => {{
            let mut complete = false;
            let mut terminal_seen = false;
            for _ in 0..16 {
                let batch = root
                    .bazaar_transport_snapshot($authority, $proof, SNAPSHOT_SOURCE, 24)
                    .await?;
                for delivery in batch.receipts {
                    let digest: [u8; 32] = Sha256::digest(delivery.body.as_bytes()).into();
                    root.bazaar_transport_ack(
                        $authority,
                        $proof,
                        DeliveryKey::Receipt {
                            issuer: delivery.issuer,
                            operation: delivery.operation,
                        },
                        digest,
                        200,
                    )
                    .await?;
                }
                for delivery in batch.publications {
                    let value: Value = serde_json::from_str(&delivery.body)?;
                    assert_eq!(value["projection_epoch"], $epoch.to_string());
                    if value["asset_kind"] == "CHARACTER" && $epoch == 2 {
                        assert_eq!(value["state"], "UNLOCKED");
                        terminal_seen = true;
                    }
                    let digest: [u8; 32] = Sha256::digest(delivery.body.as_bytes()).into();
                    root.bazaar_transport_ack(
                        $authority,
                        $proof,
                        DeliveryKey::Publication {
                            subject: delivery.subject,
                            epoch: delivery.epoch,
                            revision: delivery.revision,
                        },
                        digest,
                        200,
                    )
                    .await?;
                }
                if let Some(watermark) = batch.watermark {
                    let value: Value = serde_json::from_str(&watermark)?;
                    assert_eq!(value["projection_epoch"], $epoch.to_string());
                    complete = true;
                    break;
                }
            }
            assert!(
                complete,
                "bounded real transport drain must reach watermark"
            );
            if $epoch == 2 {
                assert!(terminal_seen, "restored terminal listings must republish");
            }
        }};
    }
    let now = now_secs()?;
    let lock_wire = v2(
        "LockBazaarCharacter",
        70,
        &character,
        r#""expected_character_revision":"1","eligibility_revision":"1","snapshot_schema_version":1"#,
        now,
    );
    allow(&root, &node, 31).await?;
    // A trust set provisioned for another recovery cannot authorize fresh capture.
    let mut wrong_generation = serde_json::to_value(&policy)?;
    wrong_generation["signing_trust_generation"] = json!(2);
    sqlx::query("UPDATE game_bazaar_configuration SET policy=$1::jsonb WHERE singleton")
        .bind(&wrong_generation)
        .execute(&pool)
        .await?;
    assert_eq!(
        decide("LockBazaarCharacter", 70, &lock_wire).await,
        Decision::Transient
    );
    sqlx::query("UPDATE game_bazaar_configuration SET policy=$1::jsonb WHERE singleton")
        .bind(serde_json::to_value(&policy)?)
        .execute(&pool)
        .await?;
    let locked = decide("LockBazaarCharacter", 70, &lock_wire).await;
    let facts = receipt(&locked, "CHAR_CMD_OK")?;
    assert_bazaar_admission_refused(&pool, &character, 31, WORLD).await?;
    let lock = facts["result_data"]["lock_id"].as_str().ok_or("lock id")?;
    assert_eq!(
        facts["result_data"]["signed_snapshot"]["snapshot"]["character_revision"],
        "2"
    );
    assert_eq!(
        facts["result_data"]["signed_snapshot"]["snapshot"]["projection_epoch"],
        "1"
    );
    assert_eq!(decide("LockBazaarCharacter", 70, &lock_wire).await, locked);
    let batch = root
        .bazaar_transport_snapshot(&authority, &node, SNAPSHOT_SOURCE, 24)
        .await?;
    let Decision::Publish {
        receipt: exact_lock_receipt,
        ..
    } = &locked
    else {
        unreachable!()
    };
    let delivered_lock = batch
        .receipts
        .iter()
        .find(|delivery| delivery.operation == u(70))
        .ok_or("Lock receipt missing from real transport")?;
    assert_eq!(delivered_lock.body.as_bytes(), exact_lock_receipt);
    let listing_status = batch
        .publications
        .iter()
        .find(|delivery| delivery.subject == format!("listing:{}", u(201)))
        .ok_or("locked listing status missing")?;
    let status: Value = serde_json::from_str(&listing_status.body)?;
    assert_eq!(status["state"], "locked");
    assert_eq!(status["listing_id"], u(201));
    assert_eq!(status["asset_id"], character);
    assert_eq!(status["custody_id"], lock);
    assert_eq!(
        status["snapshot_hash"],
        facts["result_data"]["signed_snapshot"]["snapshot_hash"]
    );
    let retry = root
        .bazaar_transport_snapshot(&authority, &node, SNAPSHOT_SOURCE, 24)
        .await?;
    assert_eq!(
        retry
            .publications
            .iter()
            .find(|delivery| delivery.subject == listing_status.subject)
            .ok_or("listing retry missing")?
            .body,
        listing_status.body
    );
    assert_eq!(
        retry
            .receipts
            .iter()
            .find(|delivery| delivery.operation == u(70))
            .ok_or("Lock retry missing")?
            .body,
        delivered_lock.body
    );
    drain_transport!(&authority, &node, 1);
    let stale = v2(
        "UnlockBazaarCharacter",
        74,
        &character,
        &format!(r#""lock_id":"{lock}","expected_lock_revision":"2""#),
        now,
    );
    receipt(
        &decide("UnlockBazaarCharacter", 74, &stale).await,
        "CHAR_CMD_BAZAAR_STALE_REVISION",
    )?;
    let listing = v1(80, &character, 31, 33, "BAZAAR_LISTING_ESCROW", now);
    let bind = v2(
        "BindBazaarCharacterTransfer",
        71,
        &character,
        &format!(
            r#""lock_id":"{lock}","expected_lock_revision":"1","transfer_operation_id":"{}","transfer_binding_digest":"{}","purpose":"BAZAAR_LISTING_ESCROW","sale_id":null,"funding":null"#,
            u(80),
            hash(listing.as_bytes())
        ),
        now,
    );
    sqlx::query("UPDATE game_bazaar_configuration SET policy=$1::jsonb WHERE singleton")
        .bind(&wrong_generation)
        .execute(&pool)
        .await?;
    assert_eq!(
        decide("BindBazaarCharacterTransfer", 71, &bind).await,
        Decision::Transient
    );
    // Immutable earlier receipts remain replayable while fresh trust is blocked.
    assert_eq!(decide("LockBazaarCharacter", 70, &lock_wire).await, locked);
    sqlx::query("UPDATE game_bazaar_configuration SET policy=$1::jsonb WHERE singleton")
        .bind(serde_json::to_value(&policy)?)
        .execute(&pool)
        .await?;
    let bound = decide("BindBazaarCharacterTransfer", 71, &bind).await;
    receipt(&bound, "CHAR_CMD_OK")?;
    assert_eq!(
        decide("BindBazaarCharacterTransfer", 71, &bind).await,
        bound
    );
    assert_eq!(
        decide(
            "BindBazaarCharacterTransfer",
            71,
            &bind.replace(&hash(listing.as_bytes()), &"0".repeat(64))
        )
        .await,
        Decision::Conflict
    );
    let ambiguous = v2(
        "UnlockBazaarCharacter",
        75,
        &character,
        &format!(r#""lock_id":"{lock}","expected_lock_revision":"2""#),
        now,
    );
    assert_eq!(
        decide("UnlockBazaarCharacter", 75, &ambiguous).await,
        Decision::Transient
    );
    allow(&root, &node, 33).await?;
    let listed = transfer(80, &listing).await;
    receipt(&listed, "CHAR_CMD_OK")?;
    assert_eq!(transfer(80, &listing).await, listed);
    let escrow_unlock = v2(
        "UnlockBazaarCharacter",
        76,
        &character,
        &format!(r#""lock_id":"{lock}","expected_lock_revision":"3""#),
        now,
    );
    assert_eq!(
        decide("UnlockBazaarCharacter", 76, &escrow_unlock).await,
        Decision::Transient
    );
    let settlement = v1(81, &character, 33, 32, "BAZAAR_SETTLEMENT", now);
    let bind_sale = v2(
        "BindBazaarCharacterTransfer",
        72,
        &character,
        &format!(
            r#""lock_id":"{lock}","expected_lock_revision":"3","transfer_operation_id":"{}","transfer_binding_digest":"{}","purpose":"BAZAAR_SETTLEMENT","sale_id":"{}","funding":{{"currency":"COINS","reservation_id":"{}","hold_revision":null,"gross_amount":"10000"}}"#,
            u(81),
            hash(settlement.as_bytes()),
            u(203),
            u(204)
        ),
        now,
    );
    receipt(
        &decide("BindBazaarCharacterTransfer", 72, &bind_sale).await,
        "CHAR_CMD_OK",
    )?;
    allow(&root, &node, 32).await?;
    let settled = transfer(81, &settlement).await;
    receipt(&settled, "CHAR_CMD_OK")?;
    assert_eq!(transfer(81, &settlement).await, settled);
    let unlock = v2(
        "UnlockBazaarCharacter",
        73,
        &character,
        &format!(r#""lock_id":"{lock}","expected_lock_revision":"5""#),
        now,
    );
    let unlocked = decide("UnlockBazaarCharacter", 73, &unlock).await;
    receipt(&unlocked, "CHAR_CMD_OK")?;
    let first_listing_history: Value =
        sqlx::query_scalar("SELECT to_jsonb(l) FROM game_bazaar_locks l WHERE lock_id=$1::uuid")
            .bind(lock)
            .fetch_one(&pool)
            .await?;
    assert_eq!(decide("UnlockBazaarCharacter", 73, &unlock).await, unlocked);
    let state=sqlx::query("SELECT account_id::text,lifecycle,character_revision::text FROM game_character_roots WHERE character_id=$1::uuid").bind(&character).fetch_one(&pool).await?;
    assert_eq!(state.try_get::<String, _>("account_id")?, u(32));
    assert_eq!(state.try_get::<i16, _>("lifecycle")?, 1);
    assert_eq!(state.try_get::<String, _>("character_revision")?, "5");
    // An unlocked Bazaar transfer still fences the former owner and wrong World.
    assert_bazaar_admission_refused(&pool, &character, 31, WORLD).await?;
    assert_bazaar_admission_refused(&pool, &character, 32, 99).await?;
    assert!(
        root.open_character_authority(&fence).await.is_ok(),
        "ownership lineage and revision chain must reload"
    );
    // Retained terminal listings are history, not a permanent prohibition on a
    // later authorized sale. Explicit cooldown zero is this fixture's policy.
    allow(&root, &node, 32).await?;
    let resale_now = now_secs()?;
    let resale_lock=v2("LockBazaarCharacter",78,&character,r#""expected_character_revision":"5","eligibility_revision":"1","snapshot_schema_version":1"#,resale_now)
        .replace(&format!(r#""seller_account_id":"{}""#,u(31)),&format!(r#""seller_account_id":"{}""#,u(32)))
        .replace(&format!(r#""listing_id":"{}""#,u(201)),&format!(r#""listing_id":"{}""#,u(205)));
    let resold_decision = decide("LockBazaarCharacter", 78, &resale_lock).await;
    let resold = receipt(&resold_decision, "CHAR_CMD_OK")?;
    let resale_lock_id = resold["result_data"]["lock_id"]
        .as_str()
        .ok_or("resale lock")?;
    let resale_wire = |command: &str, operation: u8, members: &str| {
        v2(command, operation, &character, members, resale_now)
            .replace(
                &format!(r#""seller_account_id":"{}""#, u(31)),
                &format!(r#""seller_account_id":"{}""#, u(32)),
            )
            .replace(
                &format!(r#""listing_id":"{}""#, u(201)),
                &format!(r#""listing_id":"{}""#, u(205)),
            )
    };
    let resale_escrow = v1(83, &character, 32, 33, "BAZAAR_LISTING_ESCROW", resale_now);
    let resale_bind = resale_wire(
        "BindBazaarCharacterTransfer",
        82,
        &format!(
            r#""lock_id":"{resale_lock_id}","expected_lock_revision":"1","transfer_operation_id":"{}","transfer_binding_digest":"{}","purpose":"BAZAAR_LISTING_ESCROW","sale_id":null,"funding":null"#,
            u(83),
            hash(resale_escrow.as_bytes())
        ),
    );
    receipt(
        &decide("BindBazaarCharacterTransfer", 82, &resale_bind).await,
        "CHAR_CMD_OK",
    )?;
    allow(&root, &node, 33).await?;
    receipt(&transfer(83, &resale_escrow).await, "CHAR_CMD_OK")?;
    // Revoking a capture key halts sale qualification. It does not give a
    // moderator authority to strand Game-proven custody on an escrow account.
    drain_transport!(&authority, &node, 1);
    let mut revoked = serde_json::to_value(&policy)?;
    revoked["signing_public_keys"] = json!({});
    revoked["revoked_signing_kids"] = json!(["bazaar-pg-current"]);
    sqlx::query("UPDATE game_bazaar_configuration SET policy=$1::jsonb WHERE singleton")
        .bind(&revoked)
        .execute(&pool)
        .await?;
    macro_rules! requeue_historical_lock {
        ($authority:expr, $proof:expr, $epoch:expr, $state:expr) => {{
            let remote: RemoteState = serde_json::from_value(json!({
                "command":"LockBazaarCharacter", "operation_id":u(78),
                "source_revision":"78", "state":"PENDING",
                "receipt_digest":null, "recovery_generation":null
            }))?;
            root.bazaar_reconcile_receipt($authority, $proof, &remote).await?;
            let batch = root.bazaar_transport_snapshot($authority, $proof, SNAPSHOT_SOURCE, 24).await?;
            let replay = batch.receipts.iter().find(|delivery| delivery.operation == u(78)).ok_or("historical revoked-key Lock receipt must republish")?;
            let Decision::Publish { receipt: original, .. } = &resold_decision else { unreachable!() };
            assert_eq!(replay.body.as_bytes(), original);
            let status = batch.publications.iter().find(|delivery| delivery.subject == format!("listing:{}",u(205))).ok_or("historical listing status missing")?;
            let value: Value = serde_json::from_str(&status.body)?;
            assert_eq!(value["projection_epoch"], $epoch.to_string());
            assert_eq!(value["state"], $state);
            assert_ne!(value["state"], "locked");
        }};
    }
    requeue_historical_lock!(&authority, &node, 1, "INVALID");
    let forbidden_sale = v1(85, &character, 33, 31, "BAZAAR_SETTLEMENT", resale_now);
    let forbidden_bind = resale_wire(
        "BindBazaarCharacterTransfer",
        84,
        &format!(
            r#""lock_id":"{resale_lock_id}","expected_lock_revision":"3","transfer_operation_id":"{}","transfer_binding_digest":"{}","purpose":"BAZAAR_SETTLEMENT","sale_id":"{}","funding":{{"currency":"COINS","reservation_id":"{}","hold_revision":null,"gross_amount":"10000"}}"#,
            u(85),
            hash(forbidden_sale.as_bytes()),
            u(206),
            u(207)
        ),
    );
    assert_eq!(
        decide("BindBazaarCharacterTransfer", 84, &forbidden_bind).await,
        Decision::Transient
    );
    // Compensation also remains possible when sale-only certification policy
    // becomes unavailable. The preceding negative sale changed key trust only.
    revoked["snapshot_survives_transfer"] = json!(false);
    revoked["obligations_revision"] = Value::Null;
    sqlx::query("UPDATE game_bazaar_configuration SET policy=$1::jsonb WHERE singleton")
        .bind(&revoked)
        .execute(&pool)
        .await?;
    // A current suspension narrows new acquisitions; the exact original seller
    // remains the only authorized safe-return destination.
    observe_account(&root, &node, 32, false, now_secs()?).await?;
    let returned_wire = v1(87, &character, 33, 32, "BAZAAR_ESCROW_RETURN", resale_now);
    let return_bind = resale_wire(
        "BindBazaarCharacterTransfer",
        86,
        &format!(
            r#""lock_id":"{resale_lock_id}","expected_lock_revision":"3","transfer_operation_id":"{}","transfer_binding_digest":"{}","purpose":"BAZAAR_ESCROW_RETURN","sale_id":null,"funding":null"#,
            u(87),
            hash(returned_wire.as_bytes())
        ),
    );
    receipt(
        &decide("BindBazaarCharacterTransfer", 86, &return_bind).await,
        "CHAR_CMD_OK",
    )?;
    let returned = transfer(87, &returned_wire).await;
    receipt(&returned, "CHAR_CMD_OK")?;
    assert_eq!(transfer(87, &returned_wire).await, returned);
    let resale_unlock = resale_wire(
        "UnlockBazaarCharacter",
        79,
        &format!(r#""lock_id":"{resale_lock_id}","expected_lock_revision":"5""#),
    );
    receipt(
        &decide("UnlockBazaarCharacter", 79, &resale_unlock).await,
        "CHAR_CMD_OK",
    )?;
    assert_eq!(
        decide("UnlockBazaarCharacter", 73, &unlock).await,
        unlocked,
        "old terminal listing replay remains exact"
    );
    let first_listing_after_resale: Value =
        sqlx::query_scalar("SELECT to_jsonb(l) FROM game_bazaar_locks l WHERE lock_id=$1::uuid")
            .bind(lock)
            .fetch_one(&pool)
            .await?;
    assert_eq!(first_listing_after_resale, first_listing_history);
    let retained: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_bazaar_locks WHERE character_id=$1::uuid AND state='UNLOCKED'",
    )
    .bind(&character)
    .fetch_one(&pool)
    .await?;
    assert_eq!(retained, 2);
    let successor = register(&root, 2, Some(1)).await?;
    assert_eq!(
        root.decide_bazaar_command(
            &authority,
            &node,
            SOURCE,
            "UnlockBazaarCharacter",
            &u(73),
            unlock.as_bytes()
        )
        .await,
        Decision::Transient
    );
    assert_eq!(
        root.decide_bazaar_command(
            &authority,
            &successor,
            SOURCE,
            "UnlockBazaarCharacter",
            &u(73),
            unlock.as_bytes()
        )
        .await,
        unlocked
    );
    // Revocation remains monotone after the safe compensation commits.
    assert!(
        sqlx::query("UPDATE game_bazaar_configuration SET policy=$1::jsonb WHERE singleton")
            .bind(serde_json::to_value(&policy)?)
            .execute(&pool)
            .await
            .is_err(),
        "revokedkid cannot be re-trusted"
    );
    drop(authority);
    drop(fence);
    let transition = recovery
        .begin_recovery(1, id(12), 200)
        .map_err(|error| format!("{error:?}"))?;
    root.reconcile_character_recovery(&transition)
        .await
        .map_err(|error| format!("{error:?}"))?;
    drop(transition);
    let fence = recovery
        .seal_current()
        .map_err(|error| format!("{error:?}"))?;
    let authority = root
        .open_character_authority(&fence)
        .await
        .map_err(|error| format!("{error:?}"))?;
    requeue_historical_lock!(&authority, &successor, 2, "UNLOCKED");
    // Reprovision current trust generation without resurrecting the revoked key.
    let mut current_policy: Value =
        sqlx::query_scalar("SELECT policy FROM game_bazaar_configuration WHERE singleton")
            .fetch_one(&pool)
            .await?;
    current_policy["signing_trust_generation"] = json!(2);
    current_policy["obligations_revision"] = json!(1);
    sqlx::query("UPDATE game_bazaar_configuration SET policy=$1::jsonb WHERE singleton")
        .bind(current_policy)
        .execute(&pool)
        .await?;
    assert_eq!(
        root.decide_bazaar_command(
            &authority,
            &successor,
            SOURCE,
            "UnlockBazaarCharacter",
            &u(73),
            unlock.as_bytes()
        )
        .await,
        unlocked
    );
    drain_transport!(&authority, &successor, 2);
    pool.close().await;
    drop(authority);
    drop(fence);
    drop(recovery);
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
