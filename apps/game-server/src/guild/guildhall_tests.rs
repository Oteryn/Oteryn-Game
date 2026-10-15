//! Native PG17.6 financial-component qualification, not a House auction/admission E2E.
//! Proposed guild/property/auction rows are independently installed fixture DATA.
#![allow(clippy::expect_used)]
use super::*;
use crate::durability::bank::{
    BankCoinFacts, BankDepositRequest, BankOperationIdentity, BankOperationOccurrence,
    BankOperationOutcome, BankResult,
};
use crate::durability::guild_bank::{
    GUILD_BANK_SCHEMA_PROPOSAL, GUILD_PREREQUISITE_SCHEMA_PROPOSAL,
    GUILDHALL_LEDGER_SCHEMA_PROPOSAL, GuildBankCommand, GuildBankGuildId, GuildBankIdentity,
    GuildBankRequest,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome};
use crate::guild::guildhall::{ACCOUNT_BALANCE_MAX, GUILD_BALANCE_MAX};
use crate::item_transfer_postgres_cases::{
    self as fixture, CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, fence, id,
    runtime,
};
use sqlx::Executor;

#[test]
fn split_reserve_and_lower_use_accepted_source_order() -> TestResult {
    let first = EscrowParts::default().resize(80, 60, 40)?;
    assert_eq!(
        first.after,
        EscrowParts {
            guild: 60,
            account: 20
        }
    );
    let raised = first.after.resize(90, 0, 20)?;
    assert_eq!(
        raised.after,
        EscrowParts {
            guild: 60,
            account: 30
        }
    );
    let lowered = raised.after.resize(65, 0, 10)?;
    assert_eq!(
        lowered.after,
        EscrowParts {
            guild: 60,
            account: 5
        }
    );
    let released = lowered.after.resize(0, 0, 35)?;
    assert_eq!(
        (released.guild_balance_after, released.account_balance_after),
        (60, 40)
    );
    Ok(())
}
#[test]
fn split_bounds_do_not_create_or_lose_value() -> TestResult {
    for total in [
        0,
        1,
        ACCOUNT_BALANCE_MAX,
        GUILD_BALANCE_MAX,
        ACCOUNT_BALANCE_MAX + GUILD_BALANCE_MAX,
    ] {
        let before = EscrowParts::default();
        if total == 0 {
            assert_eq!(before.resize(total, 1, 1), Err(SplitError::NoChange));
            continue;
        }
        let change = before.resize(total, GUILD_BALANCE_MAX, ACCOUNT_BALANCE_MAX)?;
        assert_eq!(change.after.total()?, total);
        assert_eq!(
            change.after.guild + change.guild_balance_after,
            GUILD_BALANCE_MAX
        );
        assert_eq!(
            change.after.account + change.account_balance_after,
            ACCOUNT_BALANCE_MAX
        );
    }
    assert_eq!(
        EscrowParts::default().resize(u64::MAX, 1, 1),
        Err(SplitError::InsufficientBalance)
    );
    assert_eq!(
        EscrowParts {
            guild: GUILD_BALANCE_MAX + 1,
            account: 0
        }
        .total(),
        Err(SplitError::InvalidStoredValue)
    );
    assert_eq!(
        EscrowParts {
            guild: 1,
            account: 0
        }
        .resize(0, GUILD_BALANCE_MAX, 0),
        Err(SplitError::BalanceLimit)
    );
    assert_eq!(
        EscrowParts::default().resize(1, 0, ACCOUNT_BALANCE_MAX + 1),
        Err(SplitError::InvalidStoredValue)
    );
    Ok(())
}

fn bank_identity(n: u8) -> GuildBankIdentity {
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
    let i = bank_identity(10);
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

// Exact private HOUSE-ACL prerequisite, fixture DATA only; independent owner remains HOUSE1a.
const PROPERTY_DEPENDENCY: &str = r#"
CREATE TABLE game_house_properties(
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),
 owner_kind TEXT,owner_character_id UUID REFERENCES game_character_roots,owner_account_id UUID,owner_guild_id UUID,
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 acl_revision NUMERIC(20,0) NOT NULL CHECK(acl_revision BETWEEN 0 AND 18446744073709551615),
 state TEXT NOT NULL CHECK(state IN('VACANT','AUCTION','OWNED','MOVE_OUT_PENDING','DISPOSITION','RETIRED')),
 content_fence_operation UUID,PRIMARY KEY(world_id,house_key),
 CHECK(((owner_kind IS NULL AND owner_character_id IS NULL AND owner_account_id IS NULL AND owner_guild_id IS NULL)
 OR (owner_kind='CHARACTER' AND owner_character_id IS NOT NULL AND owner_account_id IS NOT NULL AND owner_guild_id IS NULL)
 OR (owner_kind='GUILD' AND owner_character_id IS NULL AND owner_account_id IS NULL AND owner_guild_id IS NOT NULL)) IS TRUE));
"#;

async fn install(h: &Harness) -> TestResult {
    assert_eq!(
        sqlx::query_scalar::<_, String>("SHOW server_version_num")
            .fetch_one(&h.pool)
            .await?,
        "170006"
    );
    for schema in [
        GUILD_PREREQUISITE_SCHEMA_PROPOSAL,
        PROPERTY_DEPENDENCY,
        BID_SOURCE_SCHEMA,
        OPERATION_SCHEMA,
        GUILD_BANK_SCHEMA_PROPOSAL,
        GUILDHALL_LEDGER_SCHEMA_PROPOSAL,
        HALL_GUARDS_SCHEMA,
        AUCTION_PREREQUISITE_SCHEMA_PROPOSAL,
    ] {
        h.pool.execute(sqlx::raw_sql(schema)).await?;
    }
    sqlx::query(
        "INSERT INTO game_guilds VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,1)",
    )
    .bind(guild().as_bytes().as_slice())
    .bind(id(WORLD).as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_guild_ranks SELECT encode($1,'hex')::uuid,n FROM generate_series(1,3)n",
    )
    .bind(guild().as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_guild_members VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)",
    )
    .bind(id(CHARACTER).as_slice())
    .bind(guild().as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query("INSERT INTO game_house_properties(world_id,house_key,revision,acl_revision,state) VALUES(encode($1,'hex')::uuid,$2,1,0,'AUCTION')").bind(id(WORLD).as_slice()).bind("oteryn:content.house.fixture_hall").execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_house_auctions VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'GUILDHALL',10,0,clock_timestamp()+interval '1 day','OPEN')").bind(id(91).as_slice()).bind(id(WORLD).as_slice()).bind("oteryn:content.house.fixture_hall").execute(&h.pool).await?;
    Ok(())
}
fn step(n: u8, revision: Option<u64>, max: Option<u64>) -> TestResult<GuildhallEscrowStep> {
    let i = bank_identity(n);
    Ok(GuildhallEscrowStep {
        identity: GuildhallEscrowIdentity {
            occurrence_id: i.occurrence,
            transaction_id: i.transaction_id,
            guild_entry_id: i.guild_entry_id,
            account_entry_id: i.bank_entry_id,
        },
        bid_id: id(92),
        guild_id: *guild().as_bytes(),
        house: HouseId::new(
            crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,
            "oteryn:content.house.fixture_hall",
        )
        .ok_or("house")?,
        auction_id: id(91),
        expected_bid_revision: revision,
        max_gold: max,
    })
}
async fn footprint(h: &Harness) -> TestResult<Vec<i64>> {
    let mut values = Vec::new();
    for table in [
        "game_guildhall_operations",
        "game_guildhall_bids",
        "game_account_bank_entries",
        "game_guild_bank_entries",
    ] {
        values.push(h.count(table).await?);
    }
    values.push(
        sqlx::query_scalar(
            "SELECT coalesce(sum(balance),0)::bigint FROM game_account_bank_balances",
        )
        .fetch_one(&h.pool)
        .await?,
    );
    values.push(
        sqlx::query_scalar("SELECT coalesce(sum(balance),0)::bigint FROM game_guild_bank_balances")
            .fetch_one(&h.pool)
            .await?,
    );
    Ok(values)
}
async fn funded(h: &Harness, a: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    fund(h, a).await?;
    h.root
        .guild_bank_command(
            a,
            &h.node,
            fence()?,
            GuildBankRequest {
                identity: bank_identity(11),
                guild_id: guild(),
                command: GuildBankCommand::Deposit,
                amount: 60,
            },
        )
        .await
        .map_err(debug)?;
    Ok(())
}

#[test]
fn native_financial_split_resize_release_and_retained_replay() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guildhall_split").await?;
        let result:TestResult=async {
            install(&h).await?;
            let seal=h.recovery.seal_current().map_err(debug)?;
            let a=h.root.open_character_authority(&seal).await.map_err(debug)?;
            funded(&h,&a).await?;
            for(n,revision,max,expected)in[(20,None,70,EscrowParts{guild:60,account:20}),
                (21,Some(1),80,EscrowParts{guild:60,account:30}),(22,Some(2),55,EscrowParts{guild:60,account:5})] {
                let request=step(n,revision,Some(max))?;
                let receipt=h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),request.clone()).await.map_err(debug)?;
                assert_eq!(receipt.parts,expected);
                let before=footprint(&h).await?;
                let replay=h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),request.clone()).await.map_err(debug)?;
                assert!(replay.replayed);assert_eq!(before,footprint(&h).await?);
                let mut changed=request;changed.max_gold=Some(max+1);
                assert!(matches!(h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),changed).await,Err(GuildhallEscrowError::ConflictingOccurrence)));
            }
            // Source ownership does not follow the current leader. A release job takes no
            // Character root/session and returns the original funding Account and Guild parts.
            h.pool.execute("UPDATE game_guild_members SET rank_level=2;UPDATE game_house_auctions SET phase='RELEASING'").await?;
            let release=step(23,Some(3),None)?;
            let second_root=DurabilityRoot::connect_test_runtime(&h.database.url)?;
            assert!(second_root.maintain_ready_once().await?);
            let second_authority=second_root.open_character_authority(&seal).await.map_err(debug)?;
            let(left,right)=fixture::join_two(
                h.root.guildhall_escrow_financial_step(&a,&h.node,None,release.clone()),
                second_root.guildhall_escrow_financial_step(&second_authority,&h.node,None,release.clone()),
            ).await;
            let left=left.map_err(debug)?;let right=right.map_err(debug)?;
            assert!(left.replayed ^ right.replayed);
            assert_eq!(left.parts,EscrowParts::default());assert_eq!(right.parts,EscrowParts::default());
            let after=footprint(&h).await?;assert_eq!(&after[4..],&[40,60]);
            assert!(h.root.guildhall_escrow_financial_step(&a,&h.node,None,release).await.map_err(debug)?.replayed);
            assert!(matches!(h.root.guildhall_escrow_financial_step(&a,&h.node,None,step(24,Some(3),None)?).await,Err(GuildhallEscrowError::AlreadyReleased)));
            assert_eq!(after,footprint(&h).await?);
            // The existing bid may lower to zero when rent/current price are zero. Empty
            // custody still needs a terminal release, without inventing any ledger entry.
            h.pool.execute("UPDATE game_guild_members SET rank_level=1").await?;
            sqlx::query("INSERT INTO game_house_properties(world_id,house_key,revision,acl_revision,state) VALUES(encode($1,'hex')::uuid,'oteryn:content.house.zero_rent_hall',1,0,'AUCTION')").bind(id(WORLD).as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_house_auctions VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'oteryn:content.house.zero_rent_hall','GUILDHALL',0,0,clock_timestamp()+interval '1 day','OPEN')").bind(id(94).as_slice()).bind(id(WORLD).as_slice()).execute(&h.pool).await?;
            let mut empty=step(25,None,Some(1))?;empty.bid_id=id(93);empty.auction_id=id(94);
            empty.house=HouseId::new(empty.house.world_id(),"oteryn:content.house.zero_rent_hall").ok_or("zero rent house")?;
            h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),empty.clone()).await.map_err(debug)?;
            empty.identity=step(26,None,Some(0))?.identity;empty.expected_bid_revision=Some(1);empty.max_gold=Some(0);
            assert_eq!(h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),empty.clone()).await.map_err(debug)?.parts,EscrowParts::default());
            h.pool.execute("UPDATE game_house_auctions SET phase='RELEASING'").await?;
            empty.identity=step(27,Some(2),None)?.identity;empty.expected_bid_revision=Some(2);empty.max_gold=None;
            let before_empty_release=footprint(&h).await?;
            h.root.guildhall_escrow_financial_step(&a,&h.node,None,empty.clone()).await.map_err(debug)?;
            let after_empty_release=footprint(&h).await?;
            assert_eq!(before_empty_release[0]+1,after_empty_release[0]);
            assert_eq!(&before_empty_release[1..],&after_empty_release[1..]);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_guildhall_bids WHERE state='HELD'").fetch_one(&h.pool).await?,0);
            assert!(h.root.guildhall_escrow_financial_step(&a,&h.node,None,empty).await.map_err(debug)?.replayed);
            Ok(())
        }.await;
        let cleanup=h.cleanup().await;result?;cleanup
    })
}

#[test]
fn native_financial_current_authority_and_context_negatives_are_write_free() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guildhall_fences").await?;
        let result:TestResult=async {
            install(&h).await?;
            let seal=h.recovery.seal_current().map_err(debug)?;
            let a=h.root.open_character_authority(&seal).await.map_err(debug)?;
            funded(&h,&a).await?;
            let original=footprint(&h).await?;
            let mut stale=fence()?;stale.character_lease_generation+=1;
            assert!(matches!(h.root.guildhall_escrow_financial_step(&a,&h.node,Some(stale),step(30,None,Some(70))?).await,Err(GuildhallEscrowError::NotCurrent)));
            h.pool.execute("UPDATE game_guild_members SET rank_level=2").await?;
            assert!(matches!(h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),step(31,None,Some(70))?).await,Err(GuildhallEscrowError::NotLeader)));
            h.pool.execute("UPDATE game_guild_members SET rank_level=1;UPDATE game_guilds SET state=3").await?;
            assert!(matches!(h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),step(32,None,Some(70))?).await,Err(GuildhallEscrowError::GuildIneligible)));
            h.pool.execute("UPDATE game_guilds SET state=2;UPDATE game_house_auctions SET ends_at=clock_timestamp()-interval '1 second'").await?;
            assert!(matches!(h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),step(33,None,Some(70))?).await,Err(GuildhallEscrowError::AuctionContextUnavailable)));
            h.pool.execute("UPDATE game_house_auctions SET ends_at=clock_timestamp()+interval '1 day'").await?;
            sqlx::query("INSERT INTO game_guild_house_bans VALUES(encode($1,'hex')::uuid,clock_timestamp()+interval '1 day')").bind(guild().as_bytes().as_slice()).execute(&h.pool).await?;
            assert!(matches!(h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),step(34,None,Some(70))?).await,Err(GuildhallEscrowError::GuildIneligible)));
            assert_eq!(original,footprint(&h).await?);
            h.pool.execute("DELETE FROM game_guild_house_bans").await?;
            h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),step(35,None,Some(70))?).await.map_err(debug)?;
            h.pool.execute("UPDATE game_house_auctions SET phase='RELEASING'").await?;
            let funded_state=footprint(&h).await?;
            let wrong_secret=NodeIncarnationProof::new(h.node.fact(),crate::durability::runtime_scope_assignment::BootstrapSecret::from_bytes([99;32]));
            assert!(matches!(h.root.guildhall_escrow_financial_step(&a,&wrong_secret,None,step(36,Some(1),None)?).await,Err(GuildhallEscrowError::NotCurrent)));
            use crate::durability::runtime_scope_assignment::{RuntimeScopeAssignmentWriter,AssignmentRequest,AssignmentCommand,AssignmentPredecessor,OperationKey,ControlActor,AssignmentOutcome};
            let writer=RuntimeScopeAssignmentWriter::open(h.root.clone(),"item-transfer-writer").await.map_err(debug)?;
            // Independently install the native control permission in this isolated fixture.
            // It is not taken from the financial request/custody record or an authority bool.
            sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)")
                .bind(id(WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
            let revoked=writer.submit(&AssignmentRequest{operation_key:OperationKey::from_bytes([44;32]),actor:ControlActor::new("oteryn_test_admin").map_err(debug)?,command:AssignmentCommand::Revoke{scope:fixture::scope()?,predecessor:AssignmentPredecessor{ownership_generation:1,source_revision:1,runtime_guard_publication_revision:Some(1)}}}).await.map_err(debug)?;
            eprintln!("GUILDHALL-PG actual native revoke outcome: {revoked:?}");
            if !matches!(revoked,AssignmentOutcome::Committed(_)) {return Err(format!("native fixture revoke rejected: {revoked:?}").into())}
            // The expected immutable financial request and live node proof are unchanged;
            // independently revoking the actual assignment closes the World-job boundary.
            assert!(matches!(h.root.guildhall_escrow_financial_step(&a,&h.node,None,step(37,Some(1),None)?).await,Err(GuildhallEscrowError::NotCurrent)));
            assert_eq!(funded_state,footprint(&h).await?);
            Ok(())
        }.await;
        let cleanup=h.cleanup().await;result?;cleanup
    })
}

#[test]
fn native_financial_unbacked_custody_and_journal_rewrites_rollback() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guildhall_rollback").await?;
        let result:TestResult=async {
            install(&h).await?;
            let seal=h.recovery.seal_current().map_err(debug)?;
            let a=h.root.open_character_authority(&seal).await.map_err(debug)?;
            funded(&h,&a).await?;
            h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),step(40,None,Some(70))?).await.map_err(debug)?;
            let original=footprint(&h).await?;
            assert!(h.pool.execute("UPDATE game_guildhall_bids SET escrow_account_gold=escrow_account_gold+1,max_gold=max_gold+1,revision=revision+1").await.is_err());
            for sql in ["DELETE FROM game_guildhall_operations","UPDATE game_guildhall_operations SET request_binding=decode(repeat('00',32),'hex')","DELETE FROM game_guildhall_bids","TRUNCATE game_guildhall_bids"] {assert!(h.pool.execute(sql).await.is_err(),"{sql}");}
            assert_eq!(original,footprint(&h).await?);
            assert!(matches!(h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),step(41,Some(99),Some(80))?).await,Err(GuildhallEscrowError::StaleBid)));
            assert_eq!(original,footprint(&h).await?);
            Ok(())
        }.await;
        let cleanup=h.cleanup().await;result?;cleanup
    })
}

#[test]
fn empty_held_custody_can_terminalize_without_a_value_entry() -> TestResult {
    let change = EscrowParts::default().release(60, 40)?;
    assert_eq!(change.before, EscrowParts::default());
    assert_eq!(change.after, EscrowParts::default());
    assert_eq!(
        (change.guild_balance_before, change.guild_balance_after),
        (60, 60)
    );
    assert_eq!(
        (change.account_balance_before, change.account_balance_after),
        (40, 40)
    );
    Ok(())
}
