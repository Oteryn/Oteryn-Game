//! Fund the bank through native MINT -> pickup -> DEPOSIT, then drive the
//! Bazaar producer. No fixture disables triggers or injects a bank balance.
use super::item_transfer_postgres_cases as native;
use super::*;
use domain::currency::Coin;
use durability::bank::{
    BankCoinFacts, BankDepositRequest, BankOperationIdentity, BankOperationOccurrence,
    BankOperationOutcome, BankResult, BankWithdrawRequest,
};
use durability::item_transfer::ItemStackClass;
use native_admission_source::character_commands::Decision;

const SOURCE: &str = "oteryn:character-authority:primary";

#[test]
fn bazaar_gold_postgres_native_funds_absolute_holds_replay_release_and_bank_fence() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            (|| -> TestResult {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?
                    .block_on(async move {
                        let started = std::time::Instant::now();
                        let harness = native::Harness::create(admin, "bazaar_gold_native").await?;
                        let result = run_gold(&harness).await;
                        harness.cleanup().await?;
                        eprintln!(
                            "GPROD_BAZAAR_PG_EXECUTED native_gold_hold_release elapsed_ms={}",
                            started.elapsed().as_millis()
                        );
                        result
                    })
            })()
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Bazaar gold PostgreSQL worker failed")?
        .map_err(Into::into)
}

fn u(tag: u8) -> String {
    native::uuid_text(native::id(tag))
}
fn identity(tag: u8) -> TestResult<BankOperationIdentity> {
    Ok(BankOperationIdentity {
        occurrence: BankOperationOccurrence::from_bytes(native::id(tag)).map_err(native::debug)?,
        transaction_id: native::id(tag + 1),
        event_id: native::id(tag + 2),
        occurred_at_unix_ms: i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
        )?,
        server_build_id: "gprod-bazaar-postgres".into(),
    })
}
fn coins() -> BankCoinFacts {
    BankCoinFacts {
        gold: native::definition("Item", Coin::Gold.production_key()),
        platinum: native::definition("Item", Coin::Platinum.production_key()),
        crystal: native::definition("Item", Coin::Crystal.production_key()),
        backpack: native::backpack_facts(),
    }
}
fn committed_bank(outcome: BankOperationOutcome) -> TestResult {
    let result = match outcome {
        BankOperationOutcome::Committed(result) | BankOperationOutcome::Replayed(result) => result,
    };
    assert_eq!(result.result, BankResult::Ok);
    Ok(())
}
fn wire(command: &str, operation: u8, members: &str, now: i64) -> String {
    format!(
        r#"{{"contract_version":2,"command":"{command}","variant":"PLATFORM_BAZAAR","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","issuer_decision_id":"{}","source_revision":"{operation}","operation_id":"{}","listing_id":"{}","seller_account_id":"{}","world_id":"{}",{members},"issued_at_source":"{}","expires_at_source":"{}","audience":"OTERYN_GAME_BAZAAR_AUTHORITY"}}"#,
        u(149),
        u(operation),
        u(150),
        u(60),
        u(native::WORLD),
        now - 1,
        now + 120
    )
}
fn hold(operation: u8, amount: u64, revision: u64, now: i64) -> String {
    wire(
        "HoldBazaarGold",
        operation,
        &format!(
            r#""buyer_account_id":"{}","buyer_character_id":"{}","offer_id":"{}","hold_purpose":"OFFER","amount":"{amount}","expected_hold_revision":"{revision}""#,
            u(40),
            u(native::CHARACTER),
            u(151)
        ),
        now,
    )
}
#[track_caller]
fn receipt(decision: &Decision, code: &str) -> TestResult<serde_json::Value> {
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

async fn run_gold(h: &native::Harness) -> TestResult {
    let sealed = h.recovery.seal_current().map_err(native::debug)?;
    let authority = h
        .root
        .open_character_authority(&sealed)
        .await
        .map_err(native::debug)?;
    let backpack = h.mint(&authority, native::BACKPACK, 1).await?;
    h.committed(
        &authority,
        native::to_slot(native::command(1)?, backpack, native::backpack_facts()),
    )
    .await?;
    let crystal = h
        .mint(&authority, Coin::Crystal.production_key(), 100)
        .await?;
    let mut crystal_facts = native::facts(
        Coin::Crystal.production_key(),
        ItemStackClass::Stackable {
            proven_maximum: Some(100),
        },
    );
    crystal_facts.weight = Some(10);
    h.committed(
        &authority,
        native::to_backpack(native::command(2)?, crystal, crystal_facts),
    )
    .await?;
    committed_bank(
        h.root
            .bank_deposit(
                &authority,
                &h.node,
                native::fence()?,
                BankDepositRequest {
                    identity: identity(130)?,
                    amount: 1_000_000,
                    coins: coins(),
                    change_item_instance_ids: [native::id(133), native::id(134)],
                },
            )
            .await
            .map_err(native::debug)?,
    )?;
    let balance:i64=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid").bind(u(40)).bind(u(native::WORLD)).fetch_one(&h.pool).await?;
    assert_eq!(balance, 1_000_000);
    initialize_s2(&h.root, &h.node).await?;
    sqlx::query("UPDATE game_bazaar_configuration SET enabled=true,policy='{\"common_restore_scope\":true,\"proof_retention_seconds\":7776000,\"v2_receipt_bytes\":270336,\"gold_fee_bps\":250,\"reserve_incoming_headroom\":true}'::jsonb WHERE singleton").execute(&h.pool).await?;
    let now = now_secs()?;
    let decide = async |command: &str, operation: u8, raw: &str| {
        h.root
            .decide_bazaar_command(
                &authority,
                &h.node,
                SOURCE,
                command,
                &u(operation),
                raw.as_bytes(),
            )
            .await
    };
    allow(&h.root, &h.node, 40).await?;
    let first_raw = hold(160, 10_000, 0, now);
    let first = decide("HoldBazaarGold", 160, &first_raw).await;
    let first_receipt = receipt(&first, "CHAR_CMD_OK")?;
    let hold_id = first_receipt["result_data"]["hold_id"]
        .as_str()
        .ok_or("missing hold id")?
        .to_owned();
    assert_eq!(decide("HoldBazaarGold", 160, &first_raw).await, first);
    // Concurrent ordinary balance-write proposals cannot consume seller
    // capacity already reserved by a real, natively funded hold. Neither
    // proposal is accepted or used to fund the fixture. Require this precise
    // BEFORE guard, rather than a later missing-ledger-proof rejection.
    let seller_before: i64 = sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid")
        .bind(u(60)).bind(u(native::WORLD)).fetch_one(&h.pool).await?;
    let credit = async || {
        sqlx::query("UPDATE game_account_bank_balances SET balance=999999999999 WHERE account_id=$1::uuid AND world_id=$2::uuid")
            .bind(u(60)).bind(u(native::WORLD)).execute(&h.pool).await
    };
    let (one, two) = native::join_two(credit(), credit()).await;
    for refused in [one, two] {
        let error = refused
            .err()
            .ok_or("seller headroom credit unexpectedly committed")?;
        let sqlx::Error::Database(error) = error else {
            return Err("headroom proposal did not reach the database fence".into());
        };
        assert_eq!(error.code().as_deref(), Some("23514"));
        assert_eq!(error.message(), "Bazaar bank hold/headroom fence");
    }
    let seller_after: i64 = sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid")
        .bind(u(60)).bind(u(native::WORLD)).fetch_one(&h.pool).await?;
    assert_eq!(seller_after, seller_before);
    eprintln!(
        "GPROD_BAZAAR_HEADROOM_FENCE_EXECUTED proposed_credits=2 refused=2 balance_unchanged=true"
    );
    let reopened = DurabilityRoot::connect_test_runtime(&h.database.url)?;
    assert!(reopened.maintain_ready_once().await?);
    let reloaded_authority = reopened
        .open_character_authority(&sealed)
        .await
        .map_err(native::debug)?;
    assert_eq!(
        reopened
            .decide_bazaar_command(
                &reloaded_authority,
                &h.node,
                SOURCE,
                "HoldBazaarGold",
                &u(160),
                first_raw.as_bytes()
            )
            .await,
        first
    );
    drop(reloaded_authority);
    drop(reopened);
    assert_eq!(
        decide("HoldBazaarGold", 160, &hold(160, 20_000, 0, now)).await,
        Decision::Conflict
    );
    allow(&h.root, &h.node, 40).await?;
    receipt(
        &decide("HoldBazaarGold", 161, &hold(161, 20_000, 1, now)).await,
        "CHAR_CMD_OK",
    )?;
    let held: i64 =
        sqlx::query_scalar("SELECT amount FROM game_bazaar_gold_holds WHERE hold_id=$1::uuid")
            .bind(&hold_id)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(held, 20_000, "replacement is absolute, never additive");
    allow(&h.root, &h.node, 40).await?;
    receipt(
        &decide("HoldBazaarGold", 162, &hold(162, 25_000, 1, now)).await,
        "CHAR_CMD_BAZAAR_STALE_REVISION",
    )?;
    allow(&h.root, &h.node, 40).await?;
    receipt(
        &decide("HoldBazaarGold", 163, &hold(163, 10_000, 2, now)).await,
        "CHAR_CMD_BAZAAR_HOLD_CONFLICT",
    )?;
    let withdrawal = h
        .root
        .bank_withdraw(
            &authority,
            &h.node,
            native::fence()?,
            BankWithdrawRequest {
                identity: identity(135)?,
                amount: 1_000_000,
                coins: coins(),
                output_item_instance_ids: [native::id(138), native::id(139), native::id(140)],
            },
        )
        .await;
    match withdrawal {
        Err(_) => {}
        Ok(BankOperationOutcome::Committed(record) | BankOperationOutcome::Replayed(record)) => {
            assert_eq!(record.result, BankResult::InsufficientBalance)
        }
    }
    let unchanged:i64=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid").bind(u(40)).bind(u(native::WORLD)).fetch_one(&h.pool).await?;
    assert_eq!(unchanged, 1_000_000);
    let release = |operation: u8, amount: u64, revision: u64| {
        wire(
            "ReleaseBazaarGold",
            operation,
            &format!(
                r#""buyer_account_id":"{}","hold_id":"{hold_id}","expected_hold_revision":"{revision}","release_reason":"EXCESS","amount":"{amount}""#,
                u(40)
            ),
            now,
        )
    };
    let partial = decide("ReleaseBazaarGold", 164, &release(164, 6000, 2)).await;
    assert_eq!(
        receipt(&partial, "CHAR_CMD_OK")?["result_data"]["remaining_held"],
        "14000"
    );
    assert_eq!(
        decide("ReleaseBazaarGold", 164, &release(164, 6000, 2)).await,
        partial
    );
    let final_release = decide("ReleaseBazaarGold", 165, &release(165, 14000, 3)).await;
    assert_eq!(
        receipt(&final_release, "CHAR_CMD_OK")?["result_data"]["hold_state"],
        "RELEASED"
    );
    committed_bank(
        h.root
            .bank_withdraw(
                &authority,
                &h.node,
                native::fence()?,
                BankWithdrawRequest {
                    identity: identity(141)?,
                    amount: 1_000_000,
                    coins: coins(),
                    output_item_instance_ids: [native::id(144), native::id(145), native::id(146)],
                },
            )
            .await
            .map_err(native::debug)?,
    )?;
    let final_balance:i64=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid").bind(u(40)).bind(u(native::WORLD)).fetch_one(&h.pool).await?;
    assert_eq!(final_balance, 0);
    let decisions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_bazaar_decisions")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(decisions, 6);
    gold_delivery_sale(h, &authority, &sealed).await?;
    drop(authority);
    drop(sealed);
    Ok(())
}

async fn gold_delivery_sale(
    h: &native::Harness,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    sealed: &character_recovery_fence::SealedCharacterRecoveryFence<'_>,
) -> TestResult {
    use durability::bank::BankTransferRequest;
    use durability::item_transfer::{ItemDefinitionFacts, ItemTransferOutcome};
    use native_admission_source::character_commands::Input;
    use oteryn_gameplay_content::content::{
        ReferenceArtifactProjection, ReferencePlayableArtifactView,
    };
    use sha2::{Digest, Sha256};
    let (bytes, definitions) = super::bazaar_items::artifact()?;
    let artifact_path = std::env::temp_dir().join(format!(
        "gprod-bazaar-gold-artifact-{}-{}.bin",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::write(&artifact_path, &bytes)?;
    let result=async {
        committed_bank(h.root.bank_deposit(authority,&h.node,native::fence()?,BankDepositRequest{identity:identity(170)?,amount:1_000_000,coins:coins(),change_item_instance_ids:[native::id(173),native::id(174)]}).await.map_err(native::debug)?)?;
        allow(&h.root,&h.node,60).await?;
        let now=now_secs()?;
        let create=create_intent(190,60,native::WORLD,2,"Gold Buyer",(now-1,now+120));
        let created=h.root.decide_character_command(authority,&h.node,SOURCE,"CreateCharacter",&u(190),Input::Intent(create.as_bytes())).await;
        let created=receipt(&created,"CHAR_CMD_OK")?;
        let buyer_character=created["character_id"].as_str().ok_or("missing gold buyer Character")?.to_owned();
        committed_bank(h.root.bank_transfer(authority,&h.node,native::fence()?,BankTransferRequest{identity:identity(175)?,amount:20_000,recipient_character_id:domain::CharacterId::from_bytes(durability::character_authority::uuid_text(&buyer_character)?).map_err(native::debug)?,recipient_name_key:"goldbuyer".into()}).await.map_err(native::debug)?)?;
        let sale_item=h.mint_definition(authority,definitions[0].clone(),1).await?;
        let fence=native::fence()?;
        let request=native::to_backpack(native::command(10)?,sale_item,ItemDefinitionFacts{weight:Some(1),definition:definitions[0].clone(),stack:ItemStackClass::NonStackable,container_capacity:None,container_slot_equip_pattern:false});
        let mut candidate=h.root.freeze_item_transfer(authority,&h.node,fence,request,&super::bazaar_items::Weights).await.map_err(native::debug)?;
        let transfer=h.root.commit_item_transfer(&durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(fence.runtime_scope).await.map_err(native::debug)?,authority,&h.node,fence,&mut candidate,&super::bazaar_items::Weights).await.map_err(native::debug)?;
        assert!(matches!(transfer,ItemTransferOutcome::Committed(_)));
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,$1::uuid,$2::uuid,4)").bind(u(native::WORLD)).bind(u(native::CHANNEL)).execute(&h.pool).await?;
        let artifact=ReferencePlayableArtifactView::load(&bytes,ReferenceArtifactProjection::ServerAuthoritative)?;
        assert!(h.root.record_content_activation(&durability::content_activation::ContentActivationRequest{world_id:foundation::WorldId::decode(&native::id(native::WORLD)).map_err(native::debug)?,channel_id:foundation::ChannelId::decode(&native::id(native::CHANNEL)).map_err(native::debug)?,activation_sequence:1,previous_sequence:None,server_artifact_digest:artifact.artifact_digest(),client_artifact_digest:[3;32],frame_binding_digest:[4;32]}).await.map_err(native::debug)?);
        // The harness has no runtime actor. Close only its seeded native session
        // before exercising the offline item authority, retaining all DB guards.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE game_session_id=$1::uuid").bind(u(native::SESSION)).execute(&h.pool).await?;
        sqlx::query("UPDATE game_durability_admission_character_guards SET holder_game_session_id=NULL WHERE character_id=$1::uuid").bind(u(native::CHARACTER)).execute(&h.pool).await?;
        let hash=|b:&[u8]|Sha256::digest(b).iter().map(|v|format!("{v:02x}")).collect::<String>();
        let mut policy=durability::bazaar::Policy::default();
        policy.obligations_revision=Some(1);
        policy.inbox_capacity=Some(10000);
        policy.item_classes=vec!["nonstackable".into(),"stackable".into()];
        policy.common_restore_scope=true;
        policy.proof_retention_seconds=Some(7_776_000);
        policy.v2_receipt_bytes=Some(270336);
        policy.gold_fee_bps=Some(250);
        policy.reserve_incoming_headroom=true;
        policy.content_artifact_file=Some(artifact_path.to_str().ok_or("gold artifact path")?.to_owned());
        policy.content_artifact_sha256=Some(hash(&bytes));
        sqlx::query("UPDATE game_bazaar_configuration SET enabled=true,policy=$1::jsonb WHERE singleton").bind(serde_json::to_string(&policy)?).execute(&h.pool).await?;
        let sale_wire=|command:&str,operation:u8,members:&str|format!(r#"{{"contract_version":2,"command":"{command}","variant":"PLATFORM_BAZAAR","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","issuer_decision_id":"{}","source_revision":"{operation}","operation_id":"{}","listing_id":"{}","seller_account_id":"{}","world_id":"{}",{members},"issued_at_source":"{}","expires_at_source":"{}","audience":"OTERYN_GAME_BAZAAR_AUTHORITY"}}"#,u(199),u(operation),u(200),u(40),u(native::WORLD),now-1,now+120);
        let decide=async|command:&str,operation:u8,raw:&str|{
            h.root.decide_bazaar_command(authority,&h.node,SOURCE,command,&u(operation),raw.as_bytes()).await
        };
        allow(&h.root,&h.node,40).await?;
        let escrow=decide("EscrowBazaarItem",201,&sale_wire("EscrowBazaarItem",201,&format!(r#""seller_character_id":"{}","item_instance_id":"{}","expected_state_revision":"2","quantity":1,"transferability_revision":"1""#,u(native::CHARACTER),native::uuid_text(sale_item)))).await;
        let escrow_value=receipt(&escrow,"CHAR_CMD_OK")?;
        let escrow_id=escrow_value["result_data"]["escrow_id"].as_str().ok_or("gold escrow id")?;
        let Decision::Publish{receipt:escrow_receipt,..}=&escrow else{return Err("gold escrow receipt".into());};
        let escrow_hash=hash(escrow_receipt);
        allow(&h.root,&h.node,60).await?;
        let held=decide("HoldBazaarGold",202,&sale_wire("HoldBazaarGold",202,&format!(r#""buyer_account_id":"{}","buyer_character_id":"{buyer_character}","offer_id":"{}","hold_purpose":"OFFER","amount":"20000","expected_hold_revision":"0""#,u(60),u(203)))).await;
        let held_value=receipt(&held,"CHAR_CMD_OK")?;
        let hold_id=held_value["result_data"]["hold_id"].as_str().ok_or("sale hold id")?;
        allow(&h.root,&h.node,60).await?;
        let delivered=decide("DeliverBazaarItem",204,&sale_wire("DeliverBazaarItem",204,&format!(r#""escrow_id":"{escrow_id}","expected_escrow_revision":"1","buyer_account_id":"{}","buyer_character_id":"{buyer_character}","escrow_receipt_digest":"{escrow_hash}","sale_id":"{}","funding":{{"currency":"GOLD","reservation_id":"{hold_id}","hold_revision":"1","gross_amount":"10000"}}"#,u(60),u(205)))).await;
        receipt(&delivered,"CHAR_CMD_OK")?;
        let Decision::Publish{receipt:delivery_receipt,..}=&delivered else{return Err("gold delivery receipt".into());};
        let delivery_hash=hash(delivery_receipt);
        reject_funding_restore(h,sealed,hold_id,"ITEM").await?;
        let denied=decide("ReleaseBazaarGold",206,&sale_wire("ReleaseBazaarGold",206,&format!(r#""buyer_account_id":"{}","hold_id":"{hold_id}","expected_hold_revision":"1","release_reason":"CANCELLED","amount":"20000""#,u(60)))).await;
        receipt(&denied,"CHAR_CMD_BAZAAR_HOLD_CONFLICT")?;
        let payout_raw=sale_wire("PayoutBazaarGold",211,&format!(r#""buyer_account_id":"{}","hold_id":"{hold_id}","expected_hold_revision":"1","sale_id":"{}","gross_amount":"10000","fee_bps":250,"fee_amount":"250","seller_amount":"9750","delivery_operation_id":"{}","delivery_receipt_digest":"{delivery_hash}""#,u(60),u(205),u(204)));
        let wrong_fee=payout_raw.replace(&u(211),&u(209)).replace("\"source_revision\":\"211\"","\"source_revision\":\"209\"").replace("\"fee_amount\":\"250\"","\"fee_amount\":\"249\"").replace("\"seller_amount\":\"9750\"","\"seller_amount\":\"9751\"");
        receipt(&decide("PayoutBazaarGold",209,&wrong_fee).await,"CHAR_CMD_BAZAAR_PAYOUT_INVALID")?;
        let wrong_digest=payout_raw.replace(&u(211),&u(210)).replace("\"source_revision\":\"211\"","\"source_revision\":\"210\"").replace(&delivery_hash,&"0".repeat(64));
        receipt(&decide("PayoutBazaarGold",210,&wrong_digest).await,"CHAR_CMD_BAZAAR_HOLD_CONFLICT")?;
        let held_after_refusal:(i64,String,String)=sqlx::query_as("SELECT amount,revision::text,state FROM game_bazaar_gold_holds WHERE hold_id=$1::uuid").bind(hold_id).fetch_one(&h.pool).await?;
        assert_eq!(held_after_refusal,(20000,"1".into(),"AWAITING_PAYOUT".into()));
        let paid=decide("PayoutBazaarGold",211,&payout_raw).await;
        assert_eq!(receipt(&paid,"CHAR_CMD_OK")?["result_data"]["remaining_held"],"10000");
        assert_eq!(decide("PayoutBazaarGold",211,&payout_raw).await,paid);
        let buyer_balance:i64=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid").bind(u(60)).bind(u(native::WORLD)).fetch_one(&h.pool).await?;
        let seller_balance:i64=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid").bind(u(40)).bind(u(native::WORLD)).fetch_one(&h.pool).await?;
        let burned:i64=sqlx::query_scalar("SELECT amount FROM game_bazaar_gold_burns WHERE operation_id=$1::uuid").bind(u(211)).fetch_one(&h.pool).await?;
        assert_eq!((buyer_balance,seller_balance,burned),(10000,989750,250));
        let partial_excess=decide("ReleaseBazaarGold",212,&sale_wire("ReleaseBazaarGold",212,&format!(r#""buyer_account_id":"{}","hold_id":"{hold_id}","expected_hold_revision":"2","release_reason":"EXCESS","amount":"4000""#,u(60)))).await;
        let partial_facts=receipt(&partial_excess,"CHAR_CMD_OK")?;
        assert_eq!(partial_facts["result_data"]["hold_state"],"ACTIVE");
        assert_eq!(partial_facts["result_data"]["remaining_held"],"6000");
        let retained_paid:(String,i64,String)=sqlx::query_as("SELECT state,amount,revision::text FROM game_bazaar_gold_holds WHERE hold_id=$1::uuid").bind(hold_id).fetch_one(&h.pool).await?;
        assert_eq!(retained_paid,("PAID".into(),6000,"3".into()));
        assert!(h.root.open_character_authority(sealed).await.is_ok());
        let excess=decide("ReleaseBazaarGold",213,&sale_wire("ReleaseBazaarGold",213,&format!(r#""buyer_account_id":"{}","hold_id":"{hold_id}","expected_hold_revision":"3","release_reason":"EXCESS","amount":"6000""#,u(60)))).await;
        assert_eq!(receipt(&excess,"CHAR_CMD_OK")?["result_data"]["hold_state"],"RELEASED");
        eprintln!("GPROD_BAZAAR_PARTIAL_PAID_EXCESS_EXECUTED wire=ACTIVE internal=PAID remaining=6000 final=RELEASED");
        let lines:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_bank_entries WHERE bazaar_operation_id=$1::uuid").bind(u(211)).fetch_one(&h.pool).await?;
        assert_eq!(lines,2);
        allow(&h.root,&h.node,60).await?;
        let other=DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let other_authority=other.open_character_authority(sealed).await.map_err(native::debug)?;
        let competing=|operation:u8,offer:u8|sale_wire("HoldBazaarGold",operation,&format!(r#""buyer_account_id":"{}","buyer_character_id":"{buyer_character}","offer_id":"{}","hold_purpose":"OFFER","amount":"8000","expected_hold_revision":"0""#,u(60),u(offer))).replace(&format!("\"listing_id\":\"{}\"",u(200)),&format!("\"listing_id\":\"{}\"",u(215)));
        let one=competing(218,216);let two=competing(219,217);
        let (one,two)=native::join_two(h.root.decide_bazaar_command(authority,&h.node,SOURCE,"HoldBazaarGold",&u(218),one.as_bytes()),other.decide_bazaar_command(&other_authority,&h.node,SOURCE,"HoldBazaarGold",&u(219),two.as_bytes())).await;
        let mut winner=None;let mut committed=0;let mut refused=0;
        for outcome in [&one,&two]{
            let Decision::Publish{result_code,..}=outcome else{return Err(format!("concurrent gold reservation transient: {outcome:?}").into());};
            if result_code=="CHAR_CMD_OK"{committed+=1;winner=Some(receipt(outcome,"CHAR_CMD_OK")?);}else{assert_eq!(result_code,"CHAR_CMD_BAZAAR_HOLD_CONFLICT");refused+=1;}
        }
        assert_eq!((committed,refused),(1,1));
        let reserved:i64=sqlx::query_scalar("SELECT sum(amount)::bigint FROM game_bazaar_gold_holds WHERE buyer_account_id=$1::uuid AND world_id=$2::uuid").bind(u(60)).bind(u(native::WORLD)).fetch_one(&h.pool).await?;
        assert_eq!(reserved,8000);
        let winner=winner.ok_or("missing concurrent reservation winner")?;
        let winner_hold=winner["result_data"]["hold_id"].as_str().ok_or("winner hold id")?;
        let release=sale_wire("ReleaseBazaarGold",220,&format!(r#""buyer_account_id":"{}","hold_id":"{winner_hold}","expected_hold_revision":"1","release_reason":"EXCESS","amount":"8000""#,u(60))).replace(&format!("\"listing_id\":\"{}\"",u(200)),&format!("\"listing_id\":\"{}\"",u(215)));
        receipt(&decide("ReleaseBazaarGold",220,&release).await,"CHAR_CMD_OK")?;
        drop(other_authority);drop(other);
        eprintln!("GPROD_BAZAAR_GOLD_CONCURRENCY_EXECUTED reservations=2 committed=1 refused=1");
        gold_character_sale(h,authority,sealed,&artifact_path,&bytes).await?;
        Ok(())
    }.await;
    std::fs::remove_file(artifact_path)?;
    result
}

// Account 40's funds still come from the native deposit above. Account 60's separate
// production-created, empty Character is sold through escrow, then unlocked
// before payout: delivery proof must survive the mutable last-operation change.
async fn gold_character_sale(
    h: &native::Harness,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    sealed: &character_recovery_fence::SealedCharacterRecoveryFence<'_>,
    artifact_path: &std::path::Path,
    artifact_bytes: &[u8],
) -> TestResult {
    use native_admission_source::character_commands::Input;
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    let started = std::time::Instant::now();
    let hash = |raw: &[u8]| {
        Sha256::digest(raw)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let key_path = artifact_path.with_extension("character-gold-key");
    std::fs::write(&key_path, [29; 32])?;
    std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;
    let result = async {
        let mut policy=durability::bazaar::Policy::default();
        policy.cooldown_seconds=Some(0);
        policy.escrow_account_id=Some(u(70));
        policy.signing_key_file=Some(key_path.to_str().ok_or("gold key path")?.to_owned());
        policy.signing_kid=Some("bazaar-gold-pg".into());
        let key=ed25519_dalek::SigningKey::from_bytes(&[29;32]);
        policy.signing_public_keys.insert("bazaar-gold-pg".into(),key.verifying_key().to_bytes().iter().map(|b|format!("{b:02x}")).collect());
        policy.content_artifact_file=Some(artifact_path.to_str().ok_or("gold artifact path")?.to_owned());
        policy.content_artifact_sha256=Some(hash(artifact_bytes));
        sqlx::query("UPDATE game_bazaar_configuration SET enabled=true,policy=$1::jsonb WHERE singleton").bind(serde_json::to_string(&policy)?).execute(&h.pool).await?;
        let now=now_secs()?;
        // The earlier buyer has a legitimately delivered item in its Inbox and
        // must remain ineligible. Create a separate empty seller Character.
        allow(&h.root,&h.node,60).await?;
        let create=create_intent(244,60,native::WORLD,3,"Gold Seller",(now-1,now+120));
        let created=h.root.decide_character_command(authority,&h.node,SOURCE,"CreateCharacter",&u(244),Input::Intent(create.as_bytes())).await;
        let character=receipt(&created,"CHAR_CMD_OK")?["character_id"].as_str().ok_or("missing Gold Seller Character")?.to_owned();
        // The existing revision-one gameplay initializer is separate from the
        // Character command owner. Supply its complete typed interpretation,
        // exactly as the Character custody qualification does; keep its guard.
        sqlx::query("INSERT INTO game_character_progression_state(character_id,character_revision,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES($1::uuid,1,1,0,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')").bind(&character).execute(&h.pool).await?;
        let v2=|command:&str,operation:u8,members:&str|format!(r#"{{"contract_version":2,"command":"{command}","variant":"PLATFORM_BAZAAR","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","issuer_decision_id":"{}","source_revision":"{operation}","operation_id":"{}","listing_id":"{}","seller_account_id":"{}","world_id":"{}",{members},"issued_at_source":"{}","expires_at_source":"{}","audience":"OTERYN_GAME_BAZAAR_AUTHORITY"}}"#,u(242),u(operation),u(230),u(60),u(native::WORLD),now-1,now+120);
        let v1=|operation:u8,from:u8,to:u8,purpose:&str|format!(r#"{{"contract_version":1,"command":"TransferCharacterOwnership","variant":"PLATFORM_BAZAAR_TRANSFER","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"{}","source_revision":"{operation}","operation_id":"{}","character_id":"{character}","expected_from_account_id":"{}","to_account_id":"{}","purpose":"{purpose}","issued_at_source":"{}","expires_at_source":"{}","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}}"#,u(243),u(operation),u(from),u(to),now-1,now+120);
        let decide=async|command:&str,operation:u8,raw:&str| h.root.decide_bazaar_command(authority,&h.node,SOURCE,command,&u(operation),raw.as_bytes()).await;
        let transfer=async|operation:u8,raw:&str|h.root.decide_character_command(authority,&h.node,SOURCE,"TransferCharacterOwnership",&u(operation),Input::Intent(raw.as_bytes())).await;
        allow(&h.root,&h.node,60).await?;
        let locked=decide("LockBazaarCharacter",231,&v2("LockBazaarCharacter",231,&format!(r#""character_id":"{character}","expected_character_revision":"1","eligibility_revision":"1","snapshot_schema_version":1"#))).await;
        let lock=receipt(&locked,"CHAR_CMD_OK")?["result_data"]["lock_id"].as_str().ok_or("gold Character lock")?.to_owned();
        let listing=v1(233,60,70,"BAZAAR_LISTING_ESCROW");
        let bind=v2("BindBazaarCharacterTransfer",232,&format!(r#""character_id":"{character}","lock_id":"{lock}","expected_lock_revision":"1","transfer_operation_id":"{}","transfer_binding_digest":"{}","purpose":"BAZAAR_LISTING_ESCROW","sale_id":null,"funding":null"#,u(233),hash(listing.as_bytes())));
        receipt(&decide("BindBazaarCharacterTransfer",232,&bind).await,"CHAR_CMD_OK")?;
        allow(&h.root,&h.node,70).await?;
        receipt(&transfer(233,&listing).await,"CHAR_CMD_OK")?;
        allow(&h.root,&h.node,40).await?;
        // A current-generation terminal refusal closes only its disposition.
        // Its partially released absolute remainder must restore and be reusable.
        let failed_hold_wire=|operation:u8,amount:u64,revision:u64|v2("HoldBazaarGold",operation,&format!(r#""buyer_account_id":"{}","buyer_character_id":"{}","offer_id":"{}","hold_purpose":"OFFER","amount":"{amount}","expected_hold_revision":"{revision}""#,u(40),u(native::CHARACTER),u(253)));
        let failed=decide("HoldBazaarGold",246,&failed_hold_wire(246,10000,0)).await;
        let failed_hold=receipt(&failed,"CHAR_CMD_OK")?["result_data"]["hold_id"].as_str().ok_or("refused-sale hold")?.to_owned();
        let refused_transfer=v1(248,71,40,"BAZAAR_SETTLEMENT");
        let refused_bind=v2("BindBazaarCharacterTransfer",247,&format!(r#""character_id":"{character}","lock_id":"{lock}","expected_lock_revision":"3","transfer_operation_id":"{}","transfer_binding_digest":"{}","purpose":"BAZAAR_SETTLEMENT","sale_id":"{}","funding":{{"currency":"GOLD","reservation_id":"{failed_hold}","hold_revision":"1","gross_amount":"10000"}}"#,u(248),hash(refused_transfer.as_bytes()),u(252)));
        receipt(&decide("BindBazaarCharacterTransfer",247,&refused_bind).await,"CHAR_CMD_OK")?;
        let refused=transfer(248,&refused_transfer).await;
        receipt(&refused,"CHAR_CMD_OWNER_MISMATCH")?;
        let partial=v2("ReleaseBazaarGold",249,&format!(r#""buyer_account_id":"{}","hold_id":"{failed_hold}","expected_hold_revision":"1","release_reason":"TERMINAL_REFUSAL","amount":"6000""#,u(40)));
        let released=receipt(&decide("ReleaseBazaarGold",249,&partial).await,"CHAR_CMD_OK")?;
        assert_eq!(released["result_data"]["remaining_held"],"4000");
        let reset:(String,i64,i64,bool)=sqlx::query_as("SELECT state,amount,incoming_net,sale_id IS NULL AND gross_amount IS NULL AND delivery_operation_id IS NULL AND delivery_receipt_digest IS NULL FROM game_bazaar_gold_holds WHERE hold_id=$1::uuid").bind(&failed_hold).fetch_one(&h.pool).await?;
        assert_eq!(reset,("ACTIVE".into(),4000,3900,true));
        assert!(h.root.open_character_authority(sealed).await.is_ok());
        assert_eq!(transfer(248,&refused_transfer).await,refused);
        receipt(&decide("HoldBazaarGold",250,&failed_hold_wire(250,5000,2)).await,"CHAR_CMD_OK")?;
        let close=v2("ReleaseBazaarGold",251,&format!(r#""buyer_account_id":"{}","hold_id":"{failed_hold}","expected_hold_revision":"3","release_reason":"EXCESS","amount":"5000""#,u(40)));
        receipt(&decide("ReleaseBazaarGold",251,&close).await,"CHAR_CMD_OK")?;
        eprintln!("GPROD_BAZAAR_TERMINAL_REFUSAL_PARTIAL_RELEASE_EXECUTED remaining=4000 reusable_maximum=5000");
        let held=decide("HoldBazaarGold",234,&v2("HoldBazaarGold",234,&format!(r#""buyer_account_id":"{}","buyer_character_id":"{}","offer_id":"{}","hold_purpose":"OFFER","amount":"10000","expected_hold_revision":"0""#,u(40),u(native::CHARACTER),u(241)))).await;
        let hold=receipt(&held,"CHAR_CMD_OK")?["result_data"]["hold_id"].as_str().ok_or("Character Gold hold")?.to_owned();
        let settlement=v1(236,70,40,"BAZAAR_SETTLEMENT");
        let bind=v2("BindBazaarCharacterTransfer",235,&format!(r#""character_id":"{character}","lock_id":"{lock}","expected_lock_revision":"4","transfer_operation_id":"{}","transfer_binding_digest":"{}","purpose":"BAZAAR_SETTLEMENT","sale_id":"{}","funding":{{"currency":"GOLD","reservation_id":"{hold}","hold_revision":"1","gross_amount":"10000"}}"#,u(236),hash(settlement.as_bytes()),u(240)));
        receipt(&decide("BindBazaarCharacterTransfer",235,&bind).await,"CHAR_CMD_OK")?;
        let delivered=transfer(236,&settlement).await;
        receipt(&delivered,"CHAR_CMD_OK")?;
        let Decision::Publish{receipt:delivery_receipt,..}=&delivered else{return Err("Character sale delivery omitted receipt".into());};
        reject_funding_restore(h,sealed,&hold,"CHARACTER").await?;
        let unlock=v2("UnlockBazaarCharacter",237,&format!(r#""character_id":"{character}","lock_id":"{lock}","expected_lock_revision":"6""#));
        receipt(&decide("UnlockBazaarCharacter",237,&unlock).await,"CHAR_CMD_OK")?;
        let retained:(String,i64,i64,String)=sqlx::query_as("SELECT state,amount,incoming_net,revision::text FROM game_bazaar_gold_holds WHERE hold_id=$1::uuid").bind(&hold).fetch_one(&h.pool).await?;
        assert_eq!(retained,("AWAITING_PAYOUT".into(),10000,9750,"1".into()));
        let premature_release=v2("ReleaseBazaarGold",239,&format!(r#""buyer_account_id":"{}","hold_id":"{hold}","expected_hold_revision":"1","release_reason":"EXCESS","amount":"10000""#,u(40)));
        receipt(&decide("ReleaseBazaarGold",239,&premature_release).await,"CHAR_CMD_BAZAAR_HOLD_CONFLICT")?;
        let payout=v2("PayoutBazaarGold",238,&format!(r#""buyer_account_id":"{}","hold_id":"{hold}","expected_hold_revision":"1","sale_id":"{}","gross_amount":"10000","fee_bps":250,"fee_amount":"250","seller_amount":"9750","delivery_operation_id":"{}","delivery_receipt_digest":"{}""#,u(40),u(240),u(236),hash(delivery_receipt)));
        let paid=decide("PayoutBazaarGold",238,&payout).await;
        assert_eq!(receipt(&paid,"CHAR_CMD_OK")?["result_data"]["remaining_held"],"0");
        assert_eq!(decide("PayoutBazaarGold",238,&payout).await,paid);
        let seller_balance:i64=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid").bind(u(60)).bind(u(native::WORLD)).fetch_one(&h.pool).await?;
        assert_eq!(seller_balance,19750);
        let owner:String=sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=$1::uuid AND lifecycle=1").bind(&character).fetch_one(&h.pool).await?;
        assert_eq!(owner,u(40));
        assert!(h.root.open_character_authority(sealed).await.is_ok());
        Ok(())
    }.await;
    std::fs::remove_file(key_path)?;
    eprintln!(
        "GPROD_BAZAAR_CHARACTER_GOLD_UNLOCK_BEFORE_PAYOUT_EXECUTED elapsed_ms={} outcome={}",
        started.elapsed().as_millis(),
        if result.is_ok() { "PASS" } else { "FAIL" }
    );
    result
}

/// Simulate a partial restore in this test's isolated administrative database.
/// Normal SQL guards stay enabled. These deliberately invalid states never
/// authorize an action or seed positive funds; exact original state is restored
/// and admission is requalified before the producer continues.
async fn reject_funding_restore(
    h: &native::Harness,
    sealed: &character_recovery_fence::SealedCharacterRecoveryFence<'_>,
    hold: &str,
    asset: &str,
) -> TestResult {
    let started = std::time::Instant::now();
    let original: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(h) FROM game_bazaar_gold_holds h WHERE hold_id=$1::uuid",
    )
    .bind(hold)
    .fetch_one(&h.pool)
    .await?;
    sqlx::query("UPDATE game_bazaar_gold_holds SET state='ACTIVE',sale_id=NULL,gross_amount=NULL,delivery_operation_id=NULL,delivery_receipt_digest=NULL WHERE hold_id=$1::uuid").bind(hold).execute(&h.pool).await?;
    let rolled_back_refused = h.root.open_character_authority(sealed).await.is_err();
    sqlx::query("UPDATE game_bazaar_gold_holds h SET state=s.state,sale_id=s.sale_id,gross_amount=s.gross_amount,delivery_operation_id=s.delivery_operation_id,delivery_receipt_digest=s.delivery_receipt_digest FROM jsonb_populate_record(NULL::game_bazaar_gold_holds,$1::jsonb) s WHERE h.hold_id=s.hold_id").bind(original.to_string()).execute(&h.pool).await?;
    assert!(
        rolled_back_refused,
        "{asset} delivered custody cannot restore over a pre-handoff ACTIVE hold"
    );
    assert!(h.root.open_character_authority(sealed).await.is_ok());
    sqlx::query("DELETE FROM game_bazaar_gold_holds WHERE hold_id=$1::uuid")
        .bind(hold)
        .execute(&h.pool)
        .await?;
    let missing_refused = h.root.open_character_authority(sealed).await.is_err();
    sqlx::query("INSERT INTO game_bazaar_gold_holds SELECT (jsonb_populate_record(NULL::game_bazaar_gold_holds,$1::jsonb)).*").bind(original.to_string()).execute(&h.pool).await?;
    assert!(
        missing_refused,
        "{asset} delivered custody cannot restore without its funding hold"
    );
    assert!(h.root.open_character_authority(sealed).await.is_ok());
    eprintln!(
        "GPROD_BAZAAR_FUNDING_RESTORE_EXECUTED purpose=invalid_restore asset={asset} probes=2 refused=2 funds_accepted=0 elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
