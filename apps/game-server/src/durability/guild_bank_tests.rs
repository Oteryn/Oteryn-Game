//! Actual native Root/PG qualification. No trigger disabling or synthetic valuable funding.
//! Candidate guild lifecycle and GUILDHALL source rows are explicitly uninstalled dependencies.
#![allow(clippy::expect_used)]
use super::*;
use crate::durability::bank::{
    BankCoinFacts, BankDepositRequest, BankOperationIdentity, BankOperationOccurrence,
    BankOperationOutcome, BankResult,
};
use crate::durability::guild::{
    GUILD_GUARDS_PROPOSAL, GUILD_TABLES_PROPOSAL, GuildCommand, GuildOperationIdentity,
    GuildOutcome, GuildRequest, GuildResult, GuildTextRequest,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use crate::item_transfer_postgres_cases::{
    self as fixture, CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, fence, id,
    runtime,
};
use sqlx::Executor;

// Exact unmerged GUILD1 owner candidate, base 7229b68227a65d22bc2427545b81b61deee754a7.
// Source durability/guild.rs SHA256 33cfa4d52600ecbb4f08bc39b47d0c6f0a9754713b19090d110cfdff091d362b.
// This is a disclosed validation-overlay dependency, not a schema installed on main.
fn assert_guild_source() {
    assert_eq!(
        Sha256::digest(GUILD_TABLES_PROPOSAL.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "6d91fc5e5dbe597c72633f46dc328f30525a17498be1acbeabcc75829247343e"
    );
    assert_eq!(
        Sha256::digest(GUILD_GUARDS_PROPOSAL.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "387ba1c48521bb09a738acd6505084b65a64ec719dc4098171af5c9fb9cbb596"
    );
}

fn identity(n: u8) -> GuildBankIdentity {
    let salted = |salt| {
        let mut v = id(n);
        v[1] = salt;
        v
    };
    GuildBankIdentity {
        occurrence: salted(1),
        transaction_id: salted(2),
        guild_event_id: salted(3),
        bank_event_id: salted(4),
        guild_entry_id: salted(5),
        bank_entry_id: salted(6),
        occurred_at_unix_ms: 1_790_000_000_000,
    }
}
fn guild() -> GuildBankGuildId {
    GuildBankGuildId::from_bytes(id(90)).expect("UUIDv7")
}
fn request(n: u8, command: GuildBankCommand, amount: u64) -> GuildBankRequest {
    GuildBankRequest {
        identity: identity(n),
        guild_id: guild(),
        command,
        amount,
    }
}
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

async fn install(h: &Harness) -> TestResult {
    install_snapshot(h, 2, Some(1), true).await
}

/// Independent historical DATA is seeded before guard installation. The guard-free mode is
/// used only by the preserved financial disband test; it cannot qualify Guild lifecycle.
async fn install_snapshot(h: &Harness, state: i16, rank: Option<i16>, guards: bool) -> TestResult {
    assert_guild_source();
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        version, "170006",
        "routed qualification requires PostgreSQL17.6"
    );
    // Literal coordinated GUILDHALL source DDL is supplied by its owner in validation only.
    // This constant is not a second source owner and contains no funded/HELD fixture row.
    h.pool.execute(sqlx::raw_sql(GUILD_TABLES_PROPOSAL)).await?;
    h.pool
        .execute(sqlx::raw_sql(GUILDHALL_SOURCE_DEPENDENCY))
        .await?;
    h.pool
        .execute(sqlx::raw_sql(GUILDHALL_OPERATION_DEPENDENCY))
        .await?;
    h.pool
        .execute(sqlx::raw_sql(GUILD_BANK_SCHEMA_PROPOSAL))
        .await?;
    h.pool
        .execute(sqlx::raw_sql(GUILDHALL_LEDGER_SCHEMA_PROPOSAL))
        .await?;
    h.pool
        .execute(sqlx::raw_sql(GUILDHALL_GUARDS_DEPENDENCY))
        .await?;
    sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,vice_deficit_since,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Guild Bank Fixture',$3,9999999999999,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint,1,1)")
        .bind(guild().as_bytes().as_slice()).bind(id(WORLD).as_slice()).bind(state).execute(&h.pool).await?;
    for (level, name) in [(1_i16, "Leader"), (2, "Vice Leader"), (3, "Member")] {
        sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,$2,$3)")
            .bind(guild().as_bytes().as_slice()).bind(level).bind(name).execute(&h.pool).await?;
    }
    if let Some(rank) = rank {
        sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'',1)")
            .bind(id(CHARACTER).as_slice()).bind(guild().as_bytes().as_slice()).bind(rank).execute(&h.pool).await?;
        if rank <= 2 {
            sqlx::query("INSERT INTO game_account_guild_leadership(account_id,guild_id,character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
                .bind(id(40).as_slice()).bind(guild().as_bytes().as_slice()).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
        }
    }
    if guards {
        h.pool.execute(sqlx::raw_sql(GUILD_GUARDS_PROPOSAL)).await?;
    }
    // Qualification-only row-lock/owner-guard read privileges. Guild UPDATE first reads
    // job checkpoints before command operations; neither table is writable by this grant.
    // Genuine owner triggers still govern every mutation.
    h.pool.execute(sqlx::raw_sql("GRANT SELECT ON game_guilds,game_guild_members,game_guild_operations,game_guild_job_checkpoints,game_guild_disband_steps TO oteryn_game_runtime;GRANT UPDATE(guild_id) ON game_guilds TO oteryn_game_runtime;GRANT UPDATE(character_id) ON game_guild_members TO oteryn_game_runtime;")).await?;
    Ok(())
}

async fn fund(h: &Harness, a: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    let bag = h.mint(a, fixture::BACKPACK, 1).await?;
    let moved = h
        .transfer(
            a,
            fence()?,
            fixture::to_slot(fixture::command(1)?, bag, fixture::backpack_facts()),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(moved, ItemTransferOutcome::Committed(_)));
    let item = h.mint(a, "oteryn:item.tibia.i3031", 100).await?;
    let facts = ItemDefinitionFacts {
        definition: definition("oteryn:item.tibia.i3031"),
        stack: ItemStackClass::Stackable {
            proven_maximum: Some(100),
        },
        container_capacity: None,
        container_slot_equip_pattern: false,
    };
    let moved = h
        .transfer(
            a,
            fence()?,
            fixture::to_backpack(fixture::command(2)?, item, facts),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(moved, ItemTransferOutcome::Committed(_)));
    let i = identity(10);
    let deposited = h
        .root
        .bank_deposit(
            a,
            &h.node,
            fence()?,
            BankDepositRequest {
                identity: BankOperationIdentity {
                    occurrence: BankOperationOccurrence::from_bytes(i.occurrence).map_err(debug)?,
                    transaction_id: i.transaction_id,
                    event_id: i.bank_event_id,
                    occurred_at_unix_ms: i.occurred_at_unix_ms,
                    server_build_id: "guild-bank-native-test".into(),
                },
                amount: 100,
                coins: coins(),
                change_item_instance_ids: [i.guild_entry_id, i.bank_entry_id],
            },
        )
        .await
        .map_err(debug)?;
    assert!(matches!(deposited,BankOperationOutcome::Committed(r) if r.result==BankResult::Ok));
    Ok(())
}
async fn totals(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok((
        sqlx::query_scalar(
            "SELECT coalesce(sum(balance),0)::bigint FROM game_account_bank_balances",
        )
        .fetch_one(&h.pool)
        .await?,
        sqlx::query_scalar("SELECT coalesce(sum(balance),0)::bigint FROM game_guild_bank_balances")
            .fetch_one(&h.pool)
            .await?,
        sqlx::query_scalar("SELECT coalesce(sum(amount),0)::bigint FROM game_guild_disband_claims")
            .fetch_one(&h.pool)
            .await?,
    ))
}
async fn footprint(h: &Harness) -> TestResult<Vec<i64>> {
    let mut out = Vec::new();
    for table in [
        "game_guild_bank_operations",
        "game_guild_bank_entries",
        "game_account_bank_entries",
        "game_guild_bank_audit_proposals",
        "game_guild_disband_claims",
    ] {
        out.push(h.count(table).await?)
    }
    Ok(out)
}

#[test]
fn native_funded_transfers_conserve_and_replay_binding_is_exact() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "guild_bank_conserve").await?;
        install(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        fund(&h, &a).await?;
        for (n, amount) in [1_u64, 2, 7, 20, 30].into_iter().enumerate() {
            let req = request(20 + n as u8, GuildBankCommand::Deposit, amount);
            let first = h
                .root
                .guild_bank_command(&a, &h.node, fence()?, req.clone())
                .await
                .map_err(debug)?;
            assert!(!first.replayed);
            let fp = footprint(&h).await?;
            let replay = h
                .root
                .guild_bank_command(&a, &h.node, fence()?, req.clone())
                .await
                .map_err(debug)?;
            assert!(replay.replayed);
            assert_eq!(first.transaction_id, replay.transaction_id);
            assert_eq!(fp, footprint(&h).await?);
            let mut changed = req;
            changed.amount += 1;
            assert!(matches!(
                h.root
                    .guild_bank_command(&a, &h.node, fence()?, changed)
                    .await,
                Err(GuildBankError::ConflictingOccurrence)
            ));
            let (account, guild, claims) = totals(&h).await?;
            assert_eq!(account + guild + claims, 100);
        }
        h.root
            .guild_bank_command(
                &a,
                &h.node,
                fence()?,
                request(40, GuildBankCommand::Withdraw, 60),
            )
            .await
            .map_err(debug)?;
        assert_eq!(totals(&h).await?, (100, 0, 0));
        let events: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_guild_bank_audit_proposals")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(events, 12);
        drop(a);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn current_fences_rank_state_and_refusals_write_no_value_or_audit() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin.clone(), "guild_bank_refuse").await?;
        install_snapshot(&h, 2, Some(3), true).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        fund(&h, &a).await?;
        // Native Guild guards admit locking but reject mutation without a real owner operation.
        let mut runtime_role = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *runtime_role)
            .await?;
        let role: String = sqlx::query_scalar("SELECT current_user::text")
            .fetch_one(&mut *runtime_role)
            .await?;
        assert_eq!(role, "oteryn_game_runtime");
        sqlx::query("SELECT guild_id FROM game_guilds FOR UPDATE")
            .fetch_one(&mut *runtime_role)
            .await?;
        sqlx::query("SELECT character_id FROM game_guild_members FOR UPDATE")
            .fetch_one(&mut *runtime_role)
            .await?;
        let denied = sqlx::query("UPDATE game_guilds SET guild_id=guild_id")
            .execute(&mut *runtime_role)
            .await
            .expect_err("unbound mutation must fail the genuine Guild owner guard");
        assert_eq!(
            denied.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514"),
            "the genuine owner guard must run: {denied:?}"
        );
        assert_eq!(
            denied.as_database_error().map(|error| error.message()),
            Some("Guild mutation requires its same-transaction operation")
        );
        runtime_role.rollback().await?;
        let fp = footprint(&h).await?;
        assert!(matches!(
            h.root
                .guild_bank_command(
                    &a,
                    &h.node,
                    fence()?,
                    request(19, GuildBankCommand::Deposit, BANK_MAX)
                )
                .await,
            Err(GuildBankError::InsufficientBalance)
        ));
        assert!(matches!(
            h.root
                .guild_bank_command(
                    &a,
                    &h.node,
                    fence()?,
                    request(20, GuildBankCommand::Deposit, 101)
                )
                .await,
            Err(GuildBankError::InsufficientBalance)
        ));
        let mut stale = fence()?;
        stale.character_lease_generation = 2;
        assert!(matches!(
            h.root
                .guild_bank_command(
                    &a,
                    &h.node,
                    stale,
                    request(21, GuildBankCommand::Deposit, 1)
                )
                .await,
            Err(GuildBankError::NotCurrent)
        ));
        assert!(matches!(
            h.root
                .guild_bank_command(
                    &a,
                    &h.node,
                    fence()?,
                    request(22, GuildBankCommand::Withdraw, 1)
                )
                .await,
            Err(GuildBankError::NotAuthorized)
        ));
        // Ordinary member may deposit. State gate precedes even a membership read/refusal.
        h.root
            .guild_bank_command(
                &a,
                &h.node,
                fence()?,
                request(23, GuildBankCommand::Deposit, 1),
            )
            .await
            .map_err(debug)?;
        let changed = footprint(&h).await?;
        assert_ne!(fp, changed);
        // Separate independently seeded DISBANDING/no-member snapshot. No installed guard
        // is bypassed to manufacture a lifecycle transition the real owner does not expose.
        let disbanding = Harness::create(admin, "guild_bank_state_snapshot").await?;
        install_snapshot(&disbanding, 3, None, true).await?;
        let other_seal = disbanding.recovery.seal_current().map_err(debug)?;
        let other_a = disbanding
            .root
            .open_character_authority(&other_seal)
            .await
            .map_err(debug)?;
        let other_fp = footprint(&disbanding).await?;
        assert!(matches!(
            disbanding
                .root
                .guild_bank_command(
                    &other_a,
                    &disbanding.node,
                    fence()?,
                    request(24, GuildBankCommand::Deposit, 1)
                )
                .await,
            Err(GuildBankError::GuildDisbanding)
        ));
        assert_eq!(other_fp, footprint(&disbanding).await?);
        drop(other_a);
        drop(other_seal);
        disbanding.cleanup().await?;
        for amount in [0, BANK_MAX + 1] {
            assert!(matches!(
                h.root
                    .guild_bank_command(
                        &a,
                        &h.node,
                        fence()?,
                        request(25, GuildBankCommand::Deposit, amount)
                    )
                    .await,
                Err(GuildBankError::InvalidInput)
            ))
        }
        assert_eq!(changed, footprint(&h).await?);
        assert_eq!(totals(&h).await?, (99, 1, 0));
        drop(a);
        drop(seal);
        h.cleanup().await
    })
}

async fn payout(
    h: &Harness,
    a: &ReconciledCharacterAuthority<'_, '_>,
    i: GuildBankIdentity,
) -> TestResult<GuildBankReceipt> {
    let recovery = a.record_for(&h.root).map_err(debug)?;
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    h.root
        .try_issue_semantic_pass()?
        .run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                let receipt = payout_in_disband_transaction(&mut tx, &i, guild(), world)
                    .await
                    .map_err(|e| match e {
                        GuildBankError::Unavailable(d) => d,
                        _ => DurabilityError::InvalidStoredState,
                    })?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(receipt)
            })
        })
        .await
        .map_err(Into::into)
}

#[test]
fn disband_once_survives_member_cleanup_and_native_root_reopen() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "guild_bank_payout").await?;
        // Preserved historical financial-component test: genuine owner tables, but lifecycle
        // guards are not installed. Admin transitions below are DATA, not real disband proof.
        install_snapshot(&h, 2, Some(1), false).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        fund(&h, &a).await?;
        h.root
            .guild_bank_command(
                &a,
                &h.node,
                fence()?,
                request(20, GuildBankCommand::Deposit, 70),
            )
            .await
            .map_err(debug)?;
        sqlx::query("UPDATE game_guilds SET state=3")
            .execute(&h.pool)
            .await?;
        let receipt = payout(&h, &a, identity(30)).await?;
        assert_eq!((receipt.credited, receipt.claim_remaining), (70, 0));
        assert!(!receipt.replayed);
        assert_eq!(totals(&h).await?, (100, 0, 0));
        let fp = footprint(&h).await?;
        sqlx::query("DELETE FROM game_guild_members")
            .execute(&h.pool)
            .await?;
        sqlx::query("UPDATE game_guilds SET state=4")
            .execute(&h.pool)
            .await?;
        let replay = payout(&h, &a, identity(31)).await?;
        assert!(replay.replayed);
        assert_eq!(receipt.transaction_id, replay.transaction_id);
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let other_a = other.open_character_authority(&seal).await.map_err(debug)?;
        assert!(matches!(
            other
                .guild_bank_command(
                    &other_a,
                    &h.node,
                    fence()?,
                    request(40, GuildBankCommand::Claim, 0)
                )
                .await,
            Err(GuildBankError::BalanceLimit)
        ));
        assert_eq!(fp, footprint(&h).await?);
        let binding: (String, String, i64) = sqlx::query_as(
            "SELECT exleader_account_id::text,world_id::text,amount FROM game_guild_disband_claims",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(binding, (canonical(&id(40)), canonical(&id(WORLD)), 0));
        // This is a real retained terminal claim (not an UPDATE of an empty relation).
        assert_eq!(h.count("game_guild_disband_claims").await?, 1);
        let forged = sqlx::query("UPDATE game_guild_disband_claims SET amount=1")
            .execute(&h.pool)
            .await
            .expect_err("terminal claim cannot grow");
        assert_eq!(
            forged.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        let retained: (String, String, i64) = sqlx::query_as(
            "SELECT exleader_account_id::text,world_id::text,amount FROM game_guild_disband_claims",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(retained, binding);
        assert_eq!(totals(&h).await?, (100, 0, 0));
        drop(other_a);
        drop(other);
        drop(a);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn same_account_concurrency_serializes_with_typed_loser() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "guild_bank_race").await?;
        install(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        fund(&h, &a).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let b = other.open_character_authority(&seal).await.map_err(debug)?;
        let (one, two) = fixture::join_two(
            h.root.guild_bank_command(
                &a,
                &h.node,
                fence()?,
                request(20, GuildBankCommand::Deposit, 60),
            ),
            other.guild_bank_command(
                &b,
                &h.node,
                fence()?,
                request(21, GuildBankCommand::Deposit, 60),
            ),
        )
        .await;
        assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
        assert!(
            matches!(one, Err(GuildBankError::InsufficientBalance))
                || matches!(two, Err(GuildBankError::InsufficientBalance))
        );
        assert_eq!(totals(&h).await?, (40, 60, 0));
        assert_eq!(h.count("game_guild_bank_operations").await?, 1);
        drop(b);
        drop(other);
        drop(a);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn deferred_audit_failure_rolls_back_both_ledgers_then_same_identity_retries() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move{
        let h=Harness::create(admin,"guild_bank_rollback").await?;install(&h).await?;
        let seal=h.recovery.seal_current().map_err(debug)?;let a=h.root.open_character_authority(&seal).await.map_err(debug)?;fund(&h,&a).await?;
        let fp=footprint(&h).await?;
        h.pool.execute(sqlx::raw_sql("CREATE FUNCTION fixture_reject_guild_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected deferred audit failure' USING ERRCODE='23514'; END; $$; CREATE CONSTRAINT TRIGGER fixture_reject_guild_audit AFTER INSERT ON game_guild_bank_audit_proposals DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION fixture_reject_guild_audit();")).await?;
        let req=request(20,GuildBankCommand::Deposit,50);
        assert!(matches!(h.root.guild_bank_command(&a,&h.node,fence()?,req.clone()).await,Err(GuildBankError::Unavailable(_))));
        assert_eq!(totals(&h).await?,(100,0,0));assert_eq!(fp,footprint(&h).await?);
        h.pool.execute(sqlx::raw_sql("DROP TRIGGER fixture_reject_guild_audit ON game_guild_bank_audit_proposals; DROP FUNCTION fixture_reject_guild_audit();")).await?;
        let receipt=h.root.guild_bank_command(&a,&h.node,fence()?,req).await.map_err(debug)?;assert_eq!(receipt.transaction_id,identity(20).transaction_id);
        assert_eq!(totals(&h).await?,(50,50,0));
        // Candidate DDL retains native balance/entry guards: no value can be seeded directly.
        assert!(sqlx::query("UPDATE game_account_bank_balances SET balance=9000000000000001").execute(&h.pool).await.is_err());
        assert!(sqlx::query("UPDATE game_account_bank_balances SET balance=9000000000000000").execute(&h.pool).await.is_err());
        assert!(sqlx::query("UPDATE game_guild_bank_balances SET balance=999999999999").execute(&h.pool).await.is_err());

        assert_eq!(totals(&h).await?,(50,50,0));
        drop(a);drop(seal);h.cleanup().await
    })
}

// This is participant qualification with genuine current native Root fences and funded
// custody, not qualification of the future Hall command/auction/House authority consumer.
async fn hall_step(
    h: &Harness,
    a: &ReconciledCharacterAuthority<'_, '_>,
    reserve: bool,
) -> TestResult {
    let recovery = a.record_for(&h.root).map_err(debug)?;
    let node = h.node.clone();
    h.root.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
        let mut tx=begin_semantic_transaction(holder,deadline).await?;
        assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
        let i=identity(if reserve{50}else{51});
        occurrence_lock(&mut tx,&i.occurrence).await.map_err(|_|DurabilityError::InvalidStoredState)?;
        let f=fence().map_err(|_|DurabilityError::InvalidStoredState)?;
        assert!(assert_item_only_fence(&mut tx,&f,&node).await?.is_ok());
        sqlx::query("SELECT guild_id FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE").bind(guild().as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        sqlx::query("SELECT character_id FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid FOR UPDATE").bind(id(CHARACTER).as_slice()).fetch_one(&mut *tx).await?;
        let (gb,ga,ab,aa,eb,ea,fb,fa)=if reserve{(20_i64,0_i64,80_i64,68_i64,0_i64,20_i64,0_i64,12_i64)}else{(0,20,68,80,20,0,12,0)};
        sqlx::query("INSERT INTO game_guildhall_operations(transaction_id,occurrence_id,request_binding,kind,bid_id,guild_id,world_id,house_key,auction_id,funding_account_id,funding_character_id,guild_balance_before,guild_balance_after,account_balance_before,account_balance_after,escrow_guild_before,escrow_guild_after,escrow_account_before,escrow_account_after,guild_entry_id,account_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,'oteryn:content.house.thais_1',encode($8,'hex')::uuid,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11,$12,$13,$14,$15,$16,$17,$18,encode($19,'hex')::uuid,encode($20,'hex')::uuid)")
            .bind(i.transaction_id.as_slice()).bind(i.occurrence.as_slice()).bind(vec![0_u8;32]).bind(if reserve{"BID_RESERVE"}else{"BID_RELEASE"}).bind(id(95).as_slice()).bind(guild().as_bytes().as_slice()).bind(id(WORLD).as_slice()).bind(id(96).as_slice()).bind(id(40).as_slice()).bind(id(CHARACTER).as_slice()).bind(gb).bind(ga).bind(ab).bind(aa).bind(eb).bind(ea).bind(fb).bind(fa).bind(i.guild_entry_id.as_slice()).bind(i.bank_entry_id.as_slice()).execute(&mut *tx).await?;
        if reserve {
            sqlx::query("INSERT INTO game_guildhall_bids(bid_id,guild_id,world_id,house_key,auction_id,funding_account_id,funding_character_id,max_gold,rent_gold,escrow_guild_gold,escrow_account_gold,state,revision,last_operation_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,'oteryn:content.house.thais_1',encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,30,2,20,12,'HELD',1,encode($7,'hex')::uuid)")
                .bind(id(95).as_slice()).bind(guild().as_bytes().as_slice()).bind(id(WORLD).as_slice()).bind(id(96).as_slice()).bind(id(40).as_slice()).bind(id(CHARACTER).as_slice()).bind(i.transaction_id.as_slice()).execute(&mut *tx).await?;
        }else{
            sqlx::query("UPDATE game_guildhall_bids SET state='RELEASED',escrow_guild_gold=0,escrow_account_gold=0,revision=2,last_operation_id=encode($1,'hex')::uuid WHERE bid_id=encode($2,'hex')::uuid")
                .bind(i.transaction_id.as_slice()).bind(id(95).as_slice()).execute(&mut *tx).await?;
        }
        apply_guildhall_escrow_in_transaction(&mut tx,i.transaction_id).await.map_err(|e|match e{GuildBankError::Unavailable(d)=>d,_=>DurabilityError::InvalidStoredState})?;
        commit_semantic_transaction(tx,deadline).await?;Ok(())
    })).await.map_err(Into::into)
}

#[test]
fn native_funded_held_escrow_reserve_and_release_use_real_chains() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move{
        let h=Harness::create(admin,"guild_bank_escrow").await?;install(&h).await?;
        let seal=h.recovery.seal_current().map_err(debug)?;let a=h.root.open_character_authority(&seal).await.map_err(debug)?;fund(&h,&a).await?;
        h.root.guild_bank_command(&a,&h.node,fence()?,request(20,GuildBankCommand::Deposit,20)).await.map_err(debug)?;
        hall_step(&h,&a,true).await?;assert_eq!(totals(&h).await?,(68,0,0));
        let held:i64=sqlx::query_scalar("SELECT sum(escrow_guild_gold+escrow_account_gold)::bigint FROM game_guildhall_bids WHERE state='HELD'").fetch_one(&h.pool).await?;assert_eq!(held,32);
        hall_step(&h,&a,false).await?;assert_eq!(totals(&h).await?,(80,20,0));
        assert_eq!(h.count("game_guildhall_operations").await?,2);
        let held:i64=sqlx::query_scalar("SELECT coalesce(sum(escrow_guild_gold+escrow_account_gold),0)::bigint FROM game_guildhall_bids WHERE state='HELD'").fetch_one(&h.pool).await?;assert_eq!(held,0);
        drop(a);drop(seal);h.cleanup().await
    })
}

#[test]
fn audit_proposal_hard_ceiling_and_partial_claim_boundaries() {
    let mut v = GuildBankAuditProposal {
        transaction_id: id(1).to_vec(),
        guild_id: id(2).to_vec(),
        account_id: id(3).to_vec(),
        world_id: id(4).to_vec(),
        cause: "GUILD_DISBAND_PAYOUT".into(),
        guild_before: 70,
        guild_after: 0,
        account_before: HARD_CEILING - 20,
        account_after: HARD_CEILING,
        claim_before: 0,
        claim_after: 50,
    };
    assert!(v.checked_bytes().is_some());
    assert_eq!(headroom(v.account_before, 0).expect("headroom"), 20);
    v.account_after += 1;
    v.claim_after -= 1;
    assert!(v.checked_bytes().is_none());
    assert!(matches!(
        headroom(HARD_CEILING, 1),
        Err(GuildBankError::BalanceLimit)
    ));
    assert_eq!(
        headroom(1, HARD_CEILING - 1).expect("valid held custody"),
        0
    );
    assert_eq!(
        headroom(1, HARD_CEILING - 3).expect("valid held custody"),
        2
    );
    v.cause = "GUILD_DISBAND_CLAIM".into();
    v.guild_before = 0;
    v.account_before = HARD_CEILING - 5;
    v.account_after = HARD_CEILING;
    v.claim_before = 50;
    v.claim_after = 45;
    assert!(v.checked_bytes().is_some());
    v.claim_after = 50;
    assert!(v.checked_bytes().is_none());
    assert!(!fee_payer_is_junior(
        crate::domain::CharacterId::from_bytes(id(41)).expect("CharacterId")
    ));
}

/// P1 regression: the real owner's guarded write remains executable beside Bank. No
/// replacement Guild table or runtime-wide UPDATE-denial trigger is installed by Bank.
#[test]
fn genuine_guild_writer_and_bank_coexist_under_owner_guards() -> TestResult {
    let admin = configured_admin().ok_or("native GuildBank PG route required")?;
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "guild_bank_real_owner").await?;
        let result = async {
            install(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let a = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            fund(&h, &a).await?;
            let identity = identity(50);
            let message = GuildTextRequest::new(
                GuildRequest {
                    identity: GuildOperationIdentity {
                        occurrence: identity.occurrence,
                        transaction_id: identity.transaction_id,
                        event_id: identity.guild_event_id,
                        occurred_at_unix_ms: identity.occurred_at_unix_ms,
                    },
                    command: CommandRef::new(
                        GameSessionId::decode(&id(fixture::SESSION)).map_err(debug)?,
                        CommandId::new(50).map_err(debug)?,
                    ),
                    guild_id: crate::domain::guild::GuildId::from_bytes(*guild().as_bytes())
                        .map_err(debug)?,
                    operation: GuildCommand::SetMessage,
                },
                1,
                "Owner and Bank share the genuine Guild schema",
            )
            .map_err(debug)?;
            let outcome = h
                .root
                .commit_guild_text(&a, &h.node, fence()?, message)
                .await
                .map_err(debug)?;
            assert!(matches!(outcome, GuildOutcome::Committed(r) if r.result == GuildResult::Ok));
            let motd: String = sqlx::query_scalar("SELECT motd FROM game_guilds")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(motd, "Owner and Bank share the genuine Guild schema");
            assert_eq!(h.count("game_guild_event_outbox").await?, 1);
            let req = request(51, GuildBankCommand::Deposit, 37);
            h.root
                .guild_bank_command(&a, &h.node, fence()?, req)
                .await
                .map_err(debug)?;
            h.root
                .guild_bank_command(
                    &a,
                    &h.node,
                    fence()?,
                    request(52, GuildBankCommand::Withdraw, 37),
                )
                .await
                .map_err(debug)?;
            assert_eq!(totals(&h).await?, (100, 0, 0));
            // The original owner guard is still installed and rejects an unbound direct write.
            let denied = h
                .pool
                .execute("UPDATE game_guilds SET motd='forged'")
                .await
                .expect_err("owner guard must remain active");
            assert_eq!(
                denied.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("23514")
            );
            drop(a);
            drop(seal);
            Ok(())
        }
        .await;
        h.cleanup().await?;
        result
    })
}

/// Generated operation sequences drive the actual SQL writer, independently model balances,
/// and check conservation plus replay/refusal non-mutation at every transition. No seeded gold.
#[test]
fn generated_funded_sequences_conserve_gold_with_refusal_replay_and_conflict() -> TestResult {
    let admin = configured_admin().ok_or("native GuildBank PG route required")?;
    runtime()?.block_on(async move {
        for seed in [1_u64, 17, 0x91ab, u64::MAX] {
            let h = Harness::create(admin.clone(), "guild_bank_property").await?;
            let result: TestResult = async {
                install(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let a = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                fund(&h, &a).await?;
                let mut model = (100_u64, 0_u64);
                let mut steps = vec![
                    (GuildBankCommand::Deposit, 0),
                    (GuildBankCommand::Deposit, 101),
                    (GuildBankCommand::Withdraw, 1),
                    (GuildBankCommand::Deposit, 1),
                    (GuildBankCommand::Withdraw, 1),
                    (GuildBankCommand::Deposit, 100),
                    (GuildBankCommand::Withdraw, 100),
                    (GuildBankCommand::Deposit, BANK_MAX + 1),
                    (GuildBankCommand::Deposit, BANK_MAX),
                ];
                let mut generated = seed;
                for _ in 0..24 {
                    generated = generated
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    let command = if generated & (1 << 32) == 0 {
                        GuildBankCommand::Deposit
                    } else {
                        GuildBankCommand::Withdraw
                    };
                    steps.push((command, (generated >> 16) % 103));
                }
                let (mut committed, mut refused) = (0_u32, 0_u32);
                for (index, (command, amount)) in steps.into_iter().enumerate() {
                    let req = request(60 + u8::try_from(index)?, command, amount);
                    let before = footprint(&h).await?;
                    let balance = if command == GuildBankCommand::Deposit {
                        model.0
                    } else {
                        model.1
                    };
                    let result = h
                        .root
                        .guild_bank_command(&a, &h.node, fence()?, req.clone())
                        .await;
                    if amount == 0 || amount > BANK_MAX {
                        assert!(
                            matches!(result, Err(GuildBankError::InvalidInput)),
                            "seed={seed} step={index}"
                        );
                        refused += 1;
                        assert_eq!(before, footprint(&h).await?);
                    } else if amount > balance {
                        assert!(
                            matches!(result, Err(GuildBankError::InsufficientBalance)),
                            "seed={seed} step={index}"
                        );
                        refused += 1;
                        assert_eq!(before, footprint(&h).await?);
                    } else {
                        let receipt = result.map_err(debug)?;
                        assert!(!receipt.replayed);
                        if command == GuildBankCommand::Deposit {
                            model.0 -= amount;
                            model.1 += amount;
                        } else {
                            model.0 += amount;
                            model.1 -= amount;
                        }
                        committed += 1;
                        let after = footprint(&h).await?;
                        let replay = h
                            .root
                            .guild_bank_command(&a, &h.node, fence()?, req.clone())
                            .await
                            .map_err(debug)?;
                        assert!(replay.replayed);
                        assert_eq!(receipt.transaction_id, replay.transaction_id);
                        assert_eq!(after, footprint(&h).await?);
                        let mut conflicting = req;
                        conflicting.amount += 1;
                        assert!(matches!(
                            h.root
                                .guild_bank_command(&a, &h.node, fence()?, conflicting)
                                .await,
                            Err(GuildBankError::ConflictingOccurrence)
                        ));
                        assert_eq!(after, footprint(&h).await?);
                    }
                    let actual = totals(&h).await?;
                    assert_eq!(
                        actual,
                        (i64::try_from(model.0)?, i64::try_from(model.1)?, 0),
                        "seed={seed} step={index}"
                    );
                    assert_eq!(actual.0 + actual.1 + actual.2, 100);
                }
                assert!(committed >= 4 && refused >= 5);
                drop(a);
                drop(seal);
                Ok(())
            }
            .await;
            h.cleanup().await?;
            result?;
        }
        Ok(())
    })
}

// D1098 native participant tests obtain context from the genuine production Guild factory.
// This proves financial participation/currentness, never absent C property/Inbox completion.
async fn start_real_bank_disband(
    h: &Harness,
    a: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let i = identity(120);
    let common = GuildRequest {
        identity: GuildOperationIdentity {
            occurrence: i.occurrence,
            transaction_id: i.transaction_id,
            event_id: i.guild_event_id,
            occurred_at_unix_ms: i.occurred_at_unix_ms,
        },
        command: CommandRef::new(
            GameSessionId::decode(&id(fixture::SESSION)).map_err(debug)?,
            CommandId::new(120).map_err(debug)?,
        ),
        guild_id: crate::domain::guild::GuildId::from_bytes(*guild().as_bytes()).map_err(debug)?,
        operation: GuildCommand::Disband,
    };
    let request = crate::durability::guild::GuildDisbandRequest::new(common, "Guild Bank Fixture")
        .map_err(debug)?;
    let outcome = h
        .root
        .commit_guild_disband(a, &h.node, fence()?, request)
        .await
        .map_err(debug)?;
    assert!(matches!(outcome,GuildOutcome::Committed(r) if r.result==GuildResult::Ok));
    Ok(())
}
fn bank_disband_job(n: u8) -> TestResult<crate::durability::guild::GuildDisbandJobRequest> {
    let hall = identity(n + 1);
    Ok(crate::durability::guild::GuildDisbandJobRequest {
        world_id: WorldId::decode(&id(WORLD)).map_err(debug)?,
        guild_id: crate::domain::guild::GuildId::from_bytes(*guild().as_bytes()).map_err(debug)?,
        hall_identity: crate::durability::guildhall::GuildhallEscrowIdentity {
            occurrence_id: hall.occurrence,
            transaction_id: hall.transaction_id,
            guild_entry_id: hall.guild_entry_id,
            account_entry_id: hall.bank_entry_id,
        },
        bank_identity: identity(n),
    })
}
async fn native_controller_payout(
    pool: &sqlx::PgPool,
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    request: crate::durability::guild::GuildDisbandJobRequest,
    rollback: bool,
) -> TestResult<GuildBankDisbandPayout> {
    // Actual native fixture transaction; the genuine production factory independently
    // enforces current Root/recovery/node/World and physical xid. Guild controller tests
    // qualify the production semantic-pass wrapper; borrowed test proofs stay outside HRTB.
    let mut tx = pool.begin().await?;
    let context = crate::durability::guild::lock_guild_disband_transaction(
        &mut tx, root, authority, node, &request,
    )
    .await
    .map_err(|_| DurabilityError::InvalidStoredState)?
    .ok_or(DurabilityError::InvalidStoredState)?;
    // Every prepared identity component must equal the actual prelocked tuple.
    // No substitute class1 occurrence may be acquired after Guild/member locks.
    let before_operations: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_guild_bank_operations")
            .fetch_one(&mut *tx)
            .await?;
    for field in 0..7 {
        let mut changed = request.bank_identity.clone();
        match field {
            0 => changed.occurrence[2] ^= 1,
            1 => changed.transaction_id[2] ^= 1,
            2 => changed.guild_event_id[2] ^= 1,
            3 => changed.bank_event_id[2] ^= 1,
            4 => changed.guild_entry_id[2] ^= 1,
            5 => changed.bank_entry_id[2] ^= 1,
            _ => changed.occurred_at_unix_ms += 1,
        }
        assert!(
            matches!(
                payout_guild_disband_in_transaction(&mut tx, &context, &changed).await,
                Err(GuildBankError::NotCurrent)
            ),
            "identity field {field}"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_guild_bank_operations")
                .fetch_one(&mut *tx)
                .await?,
            before_operations
        );
    }
    let payout = payout_guild_disband_in_transaction(&mut tx, &context, &request.bank_identity)
        .await
        .map_err(|e| match e {
            GuildBankError::Unavailable(e) => e,
            _ => DurabilityError::InvalidStoredState,
        })?;
    let current = read_guild_disband_payout_in_transaction(&mut tx, &context)
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?
        .ok_or(DurabilityError::InvalidStoredState)?;
    assert_eq!(
        current.receipt.transaction_id,
        payout.receipt.transaction_id
    );
    assert_eq!(
        current.retained_claim_remaining,
        payout.retained_claim_remaining
    );
    if rollback {
        tx.rollback().await?;
    } else {
        tx.commit().await?;
    }
    Ok(payout)
}

#[test]
fn native_controller_payout_port_rolls_back_and_replays_actual_retained_state() -> TestResult {
    let admin = configured_admin().ok_or("native PostgreSQL17.6 required")?;
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "guild_bank_controller_payout").await?;
        let result: TestResult = async {
            install(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let a = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            fund(&h, &a).await?;
            h.root
                .guild_bank_command(
                    &a,
                    &h.node,
                    fence()?,
                    request(20, GuildBankCommand::Deposit, 70),
                )
                .await
                .map_err(debug)?;
            start_real_bank_disband(&h, &a).await?;
            let before = footprint(&h).await?;
            let rolled_back = native_controller_payout(
                &h.pool,
                &h.root,
                &a,
                &h.node,
                bank_disband_job(140)?,
                true,
            )
            .await?;
            assert!(!rolled_back.receipt.replayed);
            assert_eq!(footprint(&h).await?, before);
            assert_eq!(totals(&h).await?, (30, 70, 0));
            let paid = native_controller_payout(
                &h.pool,
                &h.root,
                &a,
                &h.node,
                bank_disband_job(140)?,
                false,
            )
            .await?;
            assert!(!paid.receipt.replayed);
            assert_eq!(paid.guild_debit, 70);
            assert_eq!(paid.exleader_account_id, id(40));
            assert_eq!(paid.retained_claim_remaining, 0);
            assert_eq!(totals(&h).await?, (100, 0, 0));
            let after = footprint(&h).await?;
            let reopened = DurabilityRoot::connect_test_runtime(&h.database.url)?;
            assert!(reopened.maintain_ready_once().await?);
            let current = reopened
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let replay = native_controller_payout(
                &h.pool,
                &reopened,
                &current,
                &h.node,
                bank_disband_job(142)?,
                false,
            )
            .await?;
            assert!(replay.receipt.replayed);
            assert_eq!(replay.receipt.transaction_id, paid.receipt.transaction_id);
            assert_eq!(replay.guild_debit, 70);
            assert_eq!(footprint(&h).await?, after);
            assert_eq!(h.count("game_guild_disband_claims").await?, 1);
            // This does not execute House/C closure or Guild cleanup: both retained members
            // and DISBANDING remain under the genuine owner, pending the real composer.
            assert_eq!(h.count("game_guild_members").await?, 1);
            Ok(())
        }
        .await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}

#[test]
fn native_disband_context_cannot_reauthorize_payout_in_another_physical_transaction() -> TestResult
{
    let admin = configured_admin().ok_or("native PostgreSQL17.6 required")?;
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "guild_bank_wrong_xid").await?;
        let result: TestResult = async {
            install(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let a = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            fund(&h, &a).await?;
            h.root
                .guild_bank_command(
                    &a,
                    &h.node,
                    fence()?,
                    request(20, GuildBankCommand::Deposit, 70),
                )
                .await
                .map_err(debug)?;
            start_real_bank_disband(&h, &a).await?;
            let before = footprint(&h).await?;
            let job = bank_disband_job(144)?;
            let mut tx = h.pool.begin().await?;
            let context = crate::durability::guild::lock_guild_disband_transaction(
                &mut tx, &h.root, &a, &h.node, &job,
            )
            .await
            .map_err(debug)?
            .ok_or("genuine pending Guild context required")?;
            tx.rollback().await?;
            let mut other = h.pool.begin().await?;
            assert!(matches!(
                payout_guild_disband_in_transaction(&mut other, &context, &identity(144)).await,
                Err(GuildBankError::NotCurrent)
            ));
            other.rollback().await?;
            assert_eq!(footprint(&h).await?, before);
            assert_eq!(totals(&h).await?, (30, 70, 0));
            Ok(())
        }
        .await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}

// Exact unmerged GUILDHALL owned BID_SOURCE_SCHEMA UTF8 SHA256 9fff79763a5218a304c953e7ebdc8e6449a1185dd35ac994228112838483dbbc
// Candidate composition dependency only; no admitted/installed production source.
const GUILDHALL_SOURCE_DEPENDENCY: &str = r#"
CREATE TABLE game_guildhall_bids (
 bid_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(bid_id)),
 guild_id UUID NOT NULL REFERENCES game_guilds(guild_id),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),
 auction_id UUID NOT NULL CHECK(game_character_is_uuid_v7(auction_id)),
 funding_account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 funding_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 max_gold BIGINT NOT NULL CHECK(max_gold>=0),
 rent_gold BIGINT NOT NULL CHECK(rent_gold>=0),
 escrow_guild_gold BIGINT NOT NULL CHECK(escrow_guild_gold BETWEEN 0 AND 999999999999),
 escrow_account_gold BIGINT NOT NULL CHECK(escrow_account_gold BETWEEN 0 AND 9000000000000000),
 state TEXT NOT NULL CHECK(state IN('HELD','WON','RELEASED')),
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 last_operation_id UUID NOT NULL CHECK(game_character_is_uuid_v7(last_operation_id)),
 CHECK((state='HELD' AND escrow_guild_gold::numeric+escrow_account_gold::numeric=max_gold::numeric+rent_gold::numeric)
    OR(state<>'HELD' AND escrow_guild_gold=0 AND escrow_account_gold=0))
);
CREATE UNIQUE INDEX game_guildhall_one_held_bid ON game_guildhall_bids(guild_id) WHERE state='HELD';
CREATE FUNCTION game_guildhall_bid_binding() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_guilds g WHERE g.guild_id=NEW.guild_id AND g.world_id=NEW.world_id)
 OR NOT EXISTS(SELECT 1 FROM game_character_roots c WHERE c.character_id=NEW.funding_character_id
    AND c.account_id=NEW.funding_account_id AND c.world_id=NEW.world_id) THEN
  RAISE EXCEPTION 'guildhall bid has inconsistent World/funding subject' USING ERRCODE='23514';
 END IF;
 IF TG_OP='INSERT' AND NEW.revision<>1 THEN
  RAISE EXCEPTION 'guildhall bid requires initial revision' USING ERRCODE='23514';
 END IF;
 IF TG_OP='UPDATE' AND (OLD.state<>'HELD' OR NEW.revision<>OLD.revision+1
 OR (NEW.bid_id,NEW.guild_id,NEW.world_id,NEW.house_key,NEW.auction_id,NEW.funding_account_id,NEW.funding_character_id,NEW.rent_gold)
 IS DISTINCT FROM (OLD.bid_id,OLD.guild_id,OLD.world_id,OLD.house_key,OLD.auction_id,OLD.funding_account_id,OLD.funding_character_id,OLD.rent_gold)
 OR NEW.last_operation_id=OLD.last_operation_id) THEN
  RAISE EXCEPTION 'guildhall bid requires exact immutable-source successor' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guildhall_bid_binding BEFORE INSERT OR UPDATE ON game_guildhall_bids
 FOR EACH ROW EXECUTE FUNCTION game_guildhall_bid_binding();
REVOKE ALL ON game_guildhall_bids FROM PUBLIC;
"#;

// Literal unmerged Hall operation dependency UTF8 SHA256 3113f56a8e7310b9cc2ee5780b92f3f97d56b526d905b6b63d245f454695e97e
const GUILDHALL_OPERATION_DEPENDENCY: &str = r#"
CREATE TABLE game_guildhall_operations (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 occurrence_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(occurrence_id)),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 kind TEXT NOT NULL CHECK(kind IN('BID_RESERVE','BID_LOWER','BID_RELEASE')),
 bid_id UUID NOT NULL REFERENCES game_guildhall_bids(bid_id) DEFERRABLE INITIALLY DEFERRED,
 guild_id UUID NOT NULL REFERENCES game_guilds(guild_id),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),
 auction_id UUID NOT NULL CHECK(game_character_is_uuid_v7(auction_id)),
 funding_account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 funding_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 guild_balance_before BIGINT NOT NULL CHECK(guild_balance_before BETWEEN 0 AND 999999999999),
 guild_balance_after BIGINT NOT NULL CHECK(guild_balance_after BETWEEN 0 AND 999999999999),
 account_balance_before BIGINT NOT NULL CHECK(account_balance_before BETWEEN 0 AND 9000000000000000),
 account_balance_after BIGINT NOT NULL CHECK(account_balance_after BETWEEN 0 AND 9000000000000000),
 escrow_guild_before BIGINT NOT NULL CHECK(escrow_guild_before BETWEEN 0 AND 999999999999),
 escrow_guild_after BIGINT NOT NULL CHECK(escrow_guild_after BETWEEN 0 AND 999999999999),
 escrow_account_before BIGINT NOT NULL CHECK(escrow_account_before BETWEEN 0 AND 9000000000000000),
 escrow_account_after BIGINT NOT NULL CHECK(escrow_account_after BETWEEN 0 AND 9000000000000000),
 guild_entry_id UUID NULL CHECK(guild_entry_id IS NULL OR game_character_is_uuid_v7(guild_entry_id)),
 account_entry_id UUID NULL CHECK(account_entry_id IS NULL OR game_character_is_uuid_v7(account_entry_id)),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(guild_balance_before::numeric+escrow_guild_before::numeric=guild_balance_after::numeric+escrow_guild_after::numeric),
 CHECK(account_balance_before::numeric+escrow_account_before::numeric=account_balance_after::numeric+escrow_account_after::numeric),
 CHECK((kind='BID_RESERVE' AND escrow_guild_after>=escrow_guild_before AND escrow_account_after>=escrow_account_before
    AND escrow_guild_after::numeric+escrow_account_after::numeric>escrow_guild_before::numeric+escrow_account_before::numeric)
 OR(kind IN('BID_LOWER','BID_RELEASE') AND escrow_guild_after<=escrow_guild_before AND escrow_account_after<=escrow_account_before
    AND(escrow_guild_after::numeric+escrow_account_after::numeric<escrow_guild_before::numeric+escrow_account_before::numeric
    OR(kind='BID_RELEASE' AND escrow_guild_before=0 AND escrow_account_before=0)))),
 CHECK(kind<>'BID_RELEASE' OR(escrow_guild_after=0 AND escrow_account_after=0))
);
REVOKE ALL ON game_guildhall_operations FROM PUBLIC;
"#;

// Exact unmerged Hall guard dependency UTF8 SHA256 731f76500a5ad629ed329252be31fcbfc372164271daff79329e8c1aa8cfa531
const GUILDHALL_GUARDS_DEPENDENCY: &str = r#"
CREATE UNIQUE INDEX game_guildhall_one_change_per_transaction ON game_guildhall_operations(bid_id,created_xact_id);
CREATE FUNCTION game_guildhall_custody_successor() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_guildhall_operations;
BEGIN
 SELECT * INTO o FROM game_guildhall_operations WHERE transaction_id=NEW.last_operation_id;
 IF NOT FOUND OR o.created_xact_id<>pg_current_xact_id()
 OR(o.bid_id,o.guild_id,o.world_id,o.house_key,o.auction_id,o.funding_account_id,o.funding_character_id)
 IS DISTINCT FROM(NEW.bid_id,NEW.guild_id,NEW.world_id,NEW.house_key,NEW.auction_id,NEW.funding_account_id,NEW.funding_character_id)
 OR(o.escrow_guild_after,o.escrow_account_after) IS DISTINCT FROM(NEW.escrow_guild_gold,NEW.escrow_account_gold)
 OR NEW.state<>(CASE WHEN o.kind='BID_RELEASE' THEN 'RELEASED' ELSE 'HELD' END) THEN
 RAISE EXCEPTION 'bid needs exact same-transaction custody operation' USING ERRCODE='23514';END IF;
 IF TG_OP='INSERT' THEN
 IF o.kind<>'BID_RESERVE' OR o.escrow_guild_before<>0 OR o.escrow_account_before<>0 THEN
 RAISE EXCEPTION 'initial escrow must reserve from zero' USING ERRCODE='23514';END IF;
 ELSE
 IF(o.escrow_guild_before,o.escrow_account_before) IS DISTINCT FROM(OLD.escrow_guild_gold,OLD.escrow_account_gold)
 OR(o.kind='BID_RESERVE' AND NEW.max_gold<=OLD.max_gold)
 OR(o.kind='BID_LOWER' AND NEW.max_gold>=OLD.max_gold) THEN
 RAISE EXCEPTION 'bid needs its exact escrow predecessor' USING ERRCODE='23514';END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guildhall_custody_successor BEFORE INSERT OR UPDATE ON game_guildhall_bids
 FOR EACH ROW EXECUTE FUNCTION game_guildhall_custody_successor();
CREATE FUNCTION game_guildhall_operation_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id()
 OR(NEW.guild_entry_id IS NULL)<>(NEW.guild_balance_before=NEW.guild_balance_after)
 OR(NEW.account_entry_id IS NULL)<>(NEW.account_balance_before=NEW.account_balance_after)
 OR NOT EXISTS(SELECT 1 FROM game_guildhall_bids b WHERE b.bid_id=NEW.bid_id AND b.last_operation_id=NEW.transaction_id
 AND(b.guild_id,b.world_id,b.house_key,b.auction_id,b.funding_account_id,b.funding_character_id,b.escrow_guild_gold,b.escrow_account_gold)
 IS NOT DISTINCT FROM(NEW.guild_id,NEW.world_id,NEW.house_key,NEW.auction_id,NEW.funding_account_id,NEW.funding_character_id,NEW.escrow_guild_after,NEW.escrow_account_after))
 OR(NEW.guild_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_guild_bank_entries e WHERE e.entry_id=NEW.guild_entry_id AND e.guildhall_transaction_id=NEW.transaction_id AND e.guild_id=NEW.guild_id AND e.balance_before=NEW.guild_balance_before AND e.balance_after=NEW.guild_balance_after AND e.created_xact_id=pg_current_xact_id()))
 OR(NEW.account_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries e WHERE e.entry_id=NEW.account_entry_id AND e.guildhall_transaction_id=NEW.transaction_id AND e.account_id=NEW.funding_account_id AND e.world_id=NEW.world_id AND e.balance_before=NEW.account_balance_before AND e.balance_after=NEW.account_balance_after AND e.created_xact_id=pg_current_xact_id())) THEN
 RAISE EXCEPTION 'guildhall operation requires conserved custody and both exact ledger participants' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guildhall_operation_complete AFTER INSERT ON game_guildhall_operations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guildhall_operation_complete();
CREATE TRIGGER game_guildhall_operation_immutable BEFORE UPDATE OR DELETE ON game_guildhall_operations
 FOR EACH ROW EXECUTE FUNCTION game_account_bank_immutable();
CREATE TRIGGER game_guildhall_bid_retained BEFORE DELETE ON game_guildhall_bids
 FOR EACH ROW EXECUTE FUNCTION game_account_bank_immutable();
DO $$ DECLARE t TEXT; BEGIN
 FOREACH t IN ARRAY ARRAY['game_guildhall_bids','game_guildhall_operations'] LOOP
 EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',t||'_no_truncate',t);
 END LOOP;
 FOREACH t IN ARRAY ARRAY['game_guildhall_bid_binding','game_guildhall_custody_successor','game_guildhall_operation_complete'] LOOP
 EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',t,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',t);
 END LOOP;
END $$;
GRANT SELECT,INSERT ON game_guildhall_operations TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_guildhall_bids TO oteryn_game_runtime;
GRANT SELECT ON game_guildhall_operations,game_guildhall_bids TO oteryn_game_control;
"#;
