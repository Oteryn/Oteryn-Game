// Shared genuine native NPC Bank cases: standalone target and protected PostgreSQL aggregate.
use crate::durability::DurabilityRoot;
use crate::durability::bank::*;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome};
use crate::item_transfer_postgres_cases::{
    self as fixture, Harness, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::{domain, durability, foundation};
use sqlx::Executor;

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}
fn coins() -> BankCoinFacts {
    BankCoinFacts {
        gold: definition("oteryn:item.tibia.i3031"),
        platinum: definition("oteryn:item.tibia.i3035"),
        crystal: definition("oteryn:item.tibia.i3043"),
        backpack: fixture::backpack_facts(),
    }
}
fn identity(n: u8) -> BankOperationIdentity {
    let salted = |salt| {
        let mut bytes = id(n);
        bytes[1] = salt;
        bytes
    };
    BankOperationIdentity {
        occurrence: BankOperationOccurrence::from_bytes(salted(1)).expect("UUIDv7"),
        transaction_id: salted(2),
        event_id: salted(3),
        occurred_at_unix_ms: 1_790_000_000_000,
        server_build_id: "npc-bank-native".into(),
    }
}
fn withdraw(n: u8, amount: u64) -> BankRequest {
    BankRequest::Withdraw(BankWithdrawRequest {
        identity: identity(n),
        amount,
        coins: coins(),
        output_item_instance_ids: [
            {
                let mut b = id(n);
                b[1] = 4;
                b
            },
            {
                let mut b = id(n);
                b[1] = 5;
                b
            },
            {
                let mut b = id(n);
                b[1] = 6;
                b
            },
        ],
    })
}
fn source(command: u64) -> TestResult<BankNpcSource> {
    Ok(BankNpcSource {
        command: fixture::command(command)?,
        npc_identity: id(90),
        npc_key: "oteryn:npc.paulie".into(),
        npc_generation: 1,
        catalogue_revision: 7,
        prompt_generation: 9,
        recipient_name: None,
    })
}
async fn version(h: &Harness) -> TestResult {
    let v: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(v, "170006");
    Ok(())
}
async fn fund(h: &Harness, a: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    let bag = h.mint(a, fixture::BACKPACK, 1).await?;
    assert!(matches!(
        h.transfer(
            a,
            fence()?,
            fixture::to_slot(fixture::command(1)?, bag, fixture::backpack_facts())
        )
        .await
        .map_err(debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    let gold = h.mint(a, "oteryn:item.tibia.i3031", 100).await?;
    let facts = ItemDefinitionFacts {
        definition: definition("oteryn:item.tibia.i3031"),
        stack: ItemStackClass::Stackable {
            proven_maximum: Some(100),
        },
        container_capacity: None,
        container_slot_equip_pattern: false,
    };
    h.transfer(
        a,
        fence()?,
        fixture::to_backpack(fixture::command(2)?, gold, facts),
    )
    .await
    .map_err(debug)?;
    let outcome = h
        .root
        .bank_deposit(
            a,
            &h.node,
            fence()?,
            BankDepositRequest {
                identity: identity(10),
                amount: 100,
                coins: coins(),
                change_item_instance_ids: [id(11), id(12)],
            },
        )
        .await
        .map_err(debug)?;
    assert!(matches!(outcome,BankOperationOutcome::Committed(r) if r.result==BankResult::Ok));
    Ok(())
}
async fn balance(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce(sum(balance),0)::bigint FROM game_account_bank_balances",
    )
    .fetch_one(&h.pool)
    .await?)
}
async fn footprint(h: &Harness) -> TestResult<Vec<i64>> {
    let mut v = Vec::new();
    for relation in [
        "game_npc_bank_confirmations",
        "game_account_bank_operations",
        "game_account_bank_entries",
        "game_account_bank_coin_lines",
        "game_account_bank_audit_outbox",
        "game_item_instances",
    ] {
        v.push(h.count(relation).await?);
    }
    Ok(v)
}
async fn frozen(h: &Harness) -> TestResult<Vec<u8>> {
    Ok(
        sqlx::query_scalar("SELECT frozen_request FROM game_npc_bank_confirmations")
            .fetch_one(&h.pool)
            .await?,
    )
}

#[test]
fn accepted_command_survives_no_prompt_reload_and_replays_exact_custody_once() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"npc_bank_resume").await?;version(&h).await?;
        let seal=h.recovery.seal_current().map_err(debug)?;let a=h.root.open_character_authority(&seal).await.map_err(debug)?;fund(&h,&a).await?;
        let request=withdraw(20,10);let source=source(7)?;
        h.root.enqueue_npc_bank(&a,&h.node,fence()?,source.clone(),request.clone()).await.map_err(debug)?;
        assert_eq!(balance(&h).await?,100);assert_eq!(h.count("game_account_bank_operations").await?,1);
        let bytes=frozen(&h).await?;
        // Crash-before-call: no prompt or replacement text/NPC enters the Bank owner's API.
        let other=DurabilityRoot::connect_test_runtime(&h.database.url)?;assert!(other.maintain_ready_once().await?);
        let b=other.open_character_authority(&seal).await.map_err(debug)?;
        let completion=other.resume_npc_bank(&b,&h.node,fence()?,source.command).await.map_err(debug)?.expect("original retained command");
        assert_eq!(completion.request,request);assert_eq!(completion.npc_identity,source.npc_identity);
        assert!(matches!(completion.outcome,BankOperationOutcome::Committed(r) if r.result==BankResult::Ok));
        assert_eq!(balance(&h).await?,90);assert_eq!(frozen(&h).await?,bytes);
        let fp=footprint(&h).await?;
        let replay=h.root.resume_npc_bank(&a,&h.node,fence()?,source.command).await.map_err(debug)?.expect("historical outcome");
        assert!(matches!(replay.outcome,BankOperationOutcome::Replayed(r) if r.result==BankResult::Ok));assert_eq!(fp,footprint(&h).await?);
        drop(b);drop(other);drop(a);drop(seal);h.cleanup().await
    })
}

#[test]
fn enqueue_requires_current_native_fences_and_original_command_binding() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "npc_bank_fence").await?;
        version(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        for field in [0, 1, 2] {
            let mut bad = fence()?;
            match field {
                0 => {
                    bad.connection_generation =
                        foundation::ConnectionGeneration::new(2).map_err(debug)?
                }
                1 => bad.character_lease_generation = 2,
                _ => {
                    bad.scope_ownership_generation =
                        foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?
                }
            }
            assert!(matches!(
                h.root
                    .enqueue_npc_bank(&a, &h.node, bad, source(7)?, withdraw(20, 1))
                    .await,
                Err(BankError::AuthorityRejected)
            ));
            assert_eq!(h.count("game_npc_bank_confirmations").await?, 0);
        }
        let req = withdraw(20, 1);
        let src = source(7)?;
        h.root
            .enqueue_npc_bank(&a, &h.node, fence()?, src.clone(), req.clone())
            .await
            .map_err(debug)?;
        let fp = footprint(&h).await?;
        let mut changed = req.clone();
        if let BankRequest::Withdraw(r) = &mut changed {
            r.amount = 2;
        }
        assert!(matches!(
            h.root
                .enqueue_npc_bank(&a, &h.node, fence()?, src.clone(), changed)
                .await,
            Err(BankError::ConflictingOccurrence)
        ));
        let mut changed = src.clone();
        changed.npc_generation += 1;
        assert!(matches!(
            h.root
                .enqueue_npc_bank(&a, &h.node, fence()?, changed, req)
                .await,
            Err(BankError::ConflictingOccurrence)
        ));
        assert_eq!(fp, footprint(&h).await?);
        let mut role = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *role)
            .await?;
        sqlx::query("SELECT game_session_id FROM game_npc_bank_confirmations FOR UPDATE")
            .fetch_one(&mut *role)
            .await?;
        let denied = sqlx::query("UPDATE game_npc_bank_confirmations SET frozen_request='x'")
            .execute(&mut *role)
            .await
            .expect_err("immutable request privilege");
        assert_eq!(
            denied.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("42501")
        );
        role.rollback().await?;
        let mut missing = h.pool.begin().await?;
        sqlx::query("UPDATE game_npc_bank_confirmations SET completed=true")
            .execute(&mut *missing)
            .await?;
        let denied = missing
            .commit()
            .await
            .expect_err("no fake completion without native operation");
        assert_eq!(
            denied.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        drop(a);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn failed_money_transaction_retains_original_request_without_value_then_retry() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"npc_bank_rollback").await?;version(&h).await?;
        let seal=h.recovery.seal_current().map_err(debug)?;let a=h.root.open_character_authority(&seal).await.map_err(debug)?;fund(&h,&a).await?;
        let src=source(7)?;h.root.enqueue_npc_bank(&a,&h.node,fence()?,src.clone(),withdraw(20,10)).await.map_err(debug)?;
        let before=footprint(&h).await?;let bytes=frozen(&h).await?;
        h.pool.execute(sqlx::raw_sql("CREATE FUNCTION npc_bank_test_failure() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected deferred bank audit failure' USING ERRCODE='23514'; END $$; CREATE CONSTRAINT TRIGGER npc_bank_test_failure AFTER INSERT ON game_account_bank_audit_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION npc_bank_test_failure();")).await?;
        assert!(matches!(h.root.resume_npc_bank(&a,&h.node,fence()?,src.command).await,Err(BankError::Unavailable(_))));
        assert_eq!(before,footprint(&h).await?);assert_eq!(bytes,frozen(&h).await?);assert_eq!(balance(&h).await?,100);
        let completed:bool=sqlx::query_scalar("SELECT completed FROM game_npc_bank_confirmations").fetch_one(&h.pool).await?;assert!(!completed);
        h.pool.execute(sqlx::raw_sql("DROP TRIGGER npc_bank_test_failure ON game_account_bank_audit_outbox; DROP FUNCTION npc_bank_test_failure();")).await?;
        assert!(h.root.resume_npc_bank(&a,&h.node,fence()?,src.command).await.map_err(debug)?.is_some());assert_eq!(balance(&h).await?,90);
        drop(a);drop(seal);h.cleanup().await
    })
}

#[test]
fn two_native_roots_drain_one_original_occurrence_with_exact_replay() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "npc_bank_race").await?;
        version(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        fund(&h, &a).await?;
        let src = source(7)?;
        h.root
            .enqueue_npc_bank(&a, &h.node, fence()?, src.clone(), withdraw(20, 10))
            .await
            .map_err(debug)?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let b = other.open_character_authority(&seal).await.map_err(debug)?;
        let (one, two) = fixture::join_two(
            h.root.resume_npc_bank(&a, &h.node, fence()?, src.command),
            other.resume_npc_bank(&b, &h.node, fence()?, src.command),
        )
        .await;
        let one = one.map_err(debug)?.expect("first original");
        let two = two.map_err(debug)?.expect("same original");
        let committed = usize::from(matches!(one.outcome, BankOperationOutcome::Committed(_)))
            + usize::from(matches!(two.outcome, BankOperationOutcome::Committed(_)));
        assert_eq!(committed, 1);
        assert_eq!(balance(&h).await?, 90);
        assert_eq!(h.count("game_npc_bank_confirmations").await?, 1);
        assert_eq!(h.count("game_account_bank_operations").await?, 2);
        drop(b);
        drop(other);
        drop(a);
        drop(seal);
        h.cleanup().await
    })
}

fn trade_request(n: u8, total: u64) -> TestResult<durability::npc_trade::NpcTradeRequest> {
    use crate::durability::npc_trade::*;
    let i = identity(n);
    Ok(NpcTradeRequest {
        cause: NpcTradeCause {
            npc_key: "oteryn:npc.sam".into(),
            catalogue_revision: 1,
            offer_index: 0,
            side: NpcTradeSide::Buy,
            occurrence: *i.occurrence.as_bytes(),
        },
        command: fixture::command(u64::from(n))?,
        quantity: 1,
        unit_price: total,
        offer_count: 1,
        item: ItemDefinitionFacts {
            definition: definition("oteryn:item.tibia.i3264"),
            stack: ItemStackClass::NonStackable,
            container_capacity: None,
            container_slot_equip_pattern: false,
        },
        transaction_id: i.transaction_id,
        event_id: i.event_id,
        occurred_at_unix_ms: i.occurred_at_unix_ms,
        output_item_instance_ids: [
            {
                let mut b = id(n);
                b[1] = 4;
                b
            },
            {
                let mut b = id(n);
                b[1] = 5;
                b
            },
            {
                let mut b = id(n);
                b[1] = 6;
                b
            },
        ],
        coins: NpcTradeCoins {
            gold: coins().gold,
            platinum: coins().platinum,
            crystal: coins().crystal,
        },
        backpack: fixture::backpack_facts(),
    })
}

#[test]
fn actual_npc_buy_uses_coins_then_bank_in_one_native_trade_and_single_event() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"npc_buy_bank").await?;version(&h).await?;
        let seal=h.recovery.seal_current().map_err(debug)?;let a=h.root.open_character_authority(&seal).await.map_err(debug)?;fund(&h,&a).await?;
        let request=trade_request(30,25)?;
        assert!(matches!(h.root.commit_npc_trade(&a,&h.node,fence()?,request.clone()).await,Err(durability::npc_trade::NpcTradeError::InsufficientFunds)),"V1 cannot spend Bank");
        assert_eq!(balance(&h).await?,100);
        // The exact native GOLD-FEE-ACT fixture action, independently privileged in this local DB.
        sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())").execute(&h.pool).await?;
        h.root.commit_npc_trade(&a,&h.node,fence()?,request.clone()).await.map_err(debug)?;
        assert_eq!(balance(&h).await?,75);
        let gold=h.mint(&a,"oteryn:item.tibia.i3031",10).await?;
        let facts=ItemDefinitionFacts {definition:definition("oteryn:item.tibia.i3031"),stack:ItemStackClass::Stackable{proven_maximum:Some(100)},container_capacity:None,container_slot_equip_pattern:false};
        h.transfer(&a,fence()?,fixture::to_backpack(fixture::command(3)?,gold,facts)).await.map_err(debug)?;
        let mixed=trade_request(31,40)?;h.root.commit_npc_trade(&a,&h.node,fence()?,mixed.clone()).await.map_err(debug)?;
        assert_eq!(balance(&h).await?,45);assert_eq!(h.item_state(gold).await?,(0,2));
        let summaries:Vec<(i64,i64,i64)>=sqlx::query_as("SELECT total_gold_units,burned_gold_units,bank_debit_gold_units FROM game_npc_trades ORDER BY total_gold_units").fetch_all(&h.pool).await?;
        assert_eq!(summaries,vec![(25,0,25),(40,10,30)]);
        for request in [&request,&mixed] {
            let wire:Vec<u8>=sqlx::query_scalar("SELECT envelope FROM game_item_audit_outbox WHERE transaction_id=encode($1,'hex')::uuid").bind(request.transaction_id.as_slice()).fetch_one(&h.pool).await?;
            let (_,payload)=durability::npc_trade_audit::decode_npc_trade_envelope(&wire).map_err(debug)?;
            let debit=payload.bank_debit.expect("one admitted FEE_DEBIT");
            let entry:(Vec<u8>,i64,i64,i64)=sqlx::query_as("SELECT uuid_send(entry_id),amount,balance_before,balance_after FROM game_account_bank_entries WHERE npc_trade_transaction_id=encode($1,'hex')::uuid").bind(request.transaction_id.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(entry,(debit.entry_id,debit.debit_gold_units as i64,debit.balance_before_gold_units as i64,debit.balance_after_gold_units as i64));
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=encode($1,'hex')::uuid").bind(request.transaction_id.as_slice()).fetch_one(&h.pool).await?;assert_eq!(count,1);
        }
        let fp=footprint(&h).await?;
        assert!(matches!(h.root.commit_npc_trade(&a,&h.node,fence()?,mixed).await.map_err(debug)?,durability::npc_trade::NpcTradeOutcome::AlreadyCommitted(_)));
        assert_eq!(fp,footprint(&h).await?);assert_eq!(balance(&h).await?,45);
        let before=footprint(&h).await?;
        assert!(matches!(h.root.commit_npc_trade(&a,&h.node,fence()?,trade_request(32,46)?).await,Err(durability::npc_trade::NpcTradeError::InsufficientFunds)));
        assert_eq!(before,footprint(&h).await?);
        drop(a);drop(seal);h.cleanup().await
    })
}

#[test]
fn preview_uses_actual_account_world_custody_and_private_recipient_resolution() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "npc_bank_preview").await?;
        version(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        fund(&h, &a).await?;
        let own = domain::character_name::CharacterName::parse("Fixture Hero")
            .expect("actual fixture name");
        let preview = h
            .root
            .preview_npc_bank(&a, &h.node, fence()?, Some(coins()), Some(own))
            .await
            .map_err(debug)?;
        assert_eq!(preview.balance, 100);
        assert_eq!(preview.eligible_coins, 0);
        assert_eq!(preview.recipient_refusal, Some(BankResult::SameAccount));
        assert!(preview.recipient.is_none());
        let unknown = domain::character_name::CharacterName::parse("Unknown Hero")
            .expect("valid unresolved name");
        let preview = h
            .root
            .preview_npc_bank(&a, &h.node, fence()?, None, Some(unknown))
            .await
            .map_err(debug)?;
        assert_eq!(
            preview.recipient_refusal,
            Some(BankResult::UnknownRecipient)
        );
        assert!(preview.recipient.is_none());
        let mut stale = fence()?;
        stale.character_lease_generation = 2;
        assert!(matches!(
            h.root
                .preview_npc_bank(&a, &h.node, stale, None, None)
                .await,
            Err(BankError::AuthorityRejected)
        ));
        assert_eq!(h.count("game_npc_bank_confirmations").await?, 0);
        drop(a);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn held_original_occurrence_times_out_without_value_then_locked_absence_can_resume() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"npc_bank_lock").await?;version(&h).await?;
        let seal=h.recovery.seal_current().map_err(debug)?;let a=h.root.open_character_authority(&seal).await.map_err(debug)?;fund(&h,&a).await?;
        let src=source(7)?;let req=withdraw(20,10);let occurrence=*identity(20).occurrence.as_bytes();
        h.root.enqueue_npc_bank(&a,&h.node,fence()?,src.clone(),req).await.map_err(debug)?;
        let fp=footprint(&h).await?;let bytes=frozen(&h).await?;
        let mut held=h.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:bank-operation:' || encode($1,'hex'),0))").bind(occurrence.as_slice()).execute(&mut *held).await?;
        let start=std::time::Instant::now();
        assert!(matches!(h.root.resume_npc_bank(&a,&h.node,fence()?,src.command).await,Err(BankError::Unavailable(_))));
        assert!(start.elapsed()<std::time::Duration::from_secs(5),"native two-second semantic pass stays bounded");
        assert_eq!(fp,footprint(&h).await?);assert_eq!(bytes,frozen(&h).await?);assert_eq!(balance(&h).await?,100);
        // Releasing the old transaction without a Bank operation is authoritative noncommit;
        // the native occurrence lock then serializes load-before-new-write for the same bytes.
        held.rollback().await?;
        // A timed-out native Root may need pool recovery; a separately maintained genuine Root
        // independently reconciles recovery/current fences before draining the same frozen work.
        let other=DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let b=other.open_character_authority(&seal).await.map_err(debug)?;
        let done=other.resume_npc_bank(&b,&h.node,fence()?,src.command).await.map_err(debug)?.expect("retained original");
        assert!(matches!(done.outcome,BankOperationOutcome::Committed(r) if r.result==BankResult::Ok));assert_eq!(balance(&h).await?,90);
        let fp=footprint(&h).await?;
        let replay=other.resume_npc_bank(&b,&h.node,fence()?,src.command).await.map_err(debug)?.expect("committed original");
        assert!(matches!(replay.outcome,BankOperationOutcome::Replayed(_)));assert_eq!(fp,footprint(&h).await?);
        drop(b);drop(other);drop(a);drop(seal);h.cleanup().await
    })
}

async fn wait_for_blocked(h: &Harness, minimum: i64) -> TestResult {
    for _ in 0..80 {
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM pg_locks WHERE database=(SELECT oid FROM pg_database WHERE datname=current_database()) AND NOT granted").fetch_one(&h.pool).await?;
        if count >= minimum {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    Err("native transaction did not reach the bounded fixture barrier".into())
}

#[test]
fn inflight_native_bank_commit_is_replayed_before_any_new_money_execution() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"npc_bank_inflight").await?;version(&h).await?;
        let seal=h.recovery.seal_current().map_err(debug)?;let a=h.root.open_character_authority(&seal).await.map_err(debug)?;fund(&h,&a).await?;
        let src=source(7)?;let req=withdraw(20,10);
        h.root.enqueue_npc_bank(&a,&h.node,fence()?,src.clone(),req.clone()).await.map_err(debug)?;
        // A test-only scheduling barrier adds no grant and disables no native guard. The actual
        // independently maintained Root still runs every recovery/session/node/custody proof.
        h.pool.execute(sqlx::raw_sql("CREATE FUNCTION npc_bank_test_barrier() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(314159265); RETURN NEW; END $$; CREATE TRIGGER npc_bank_test_barrier BEFORE INSERT ON game_account_bank_operations FOR EACH ROW EXECUTE FUNCTION npc_bank_test_barrier();")).await?;
        let mut held=h.pool.begin().await?;sqlx::query("SELECT pg_advisory_xact_lock(314159265)").execute(&mut *held).await?;
        let other=DurabilityRoot::connect_test_runtime(&h.database.url)?;assert!(other.maintain_ready_once().await?);let b=other.open_character_authority(&seal).await.map_err(debug)?;
        let BankRequest::Withdraw(request)=req else{return Err("fixture request kind".into())};
        let writer=other.bank_withdraw(&b,&h.node,fence()?,request);
        let reconciliation=async {
            wait_for_blocked(&h,1).await?; // actual writer holds occurrence, waits at INSERT
            let resume=h.root.resume_npc_bank(&a,&h.node,fence()?,src.command);
            let release=async {wait_for_blocked(&h,2).await?;held.rollback().await?;Ok::<(),Box<dyn std::error::Error>>(())};
            let (outcome,released)=fixture::join_two(resume,release).await;released?;
            outcome.map_err(|e| -> Box<dyn std::error::Error> { debug(e).into() })
        };
        let (written,replayed)=fixture::join_two(writer,reconciliation).await;
        assert!(matches!(written.map_err(debug)?,BankOperationOutcome::Committed(r) if r.result==BankResult::Ok));
        assert!(matches!(replayed?.expect("same retained command").outcome,BankOperationOutcome::Replayed(r) if r.result==BankResult::Ok));
        assert_eq!(balance(&h).await?,90);assert_eq!(h.count("game_account_bank_operations").await?,2);
        drop(b);drop(other);drop(a);drop(seal);h.cleanup().await
    })
}
