//! Native PG17.6 financial-component qualification, not a House auction/admission E2E.
//! Exact genuine Guild owner tables/guards; House property/auction context remains fixture DATA.
#![allow(clippy::expect_used)]
use super::*;
use crate::durability::bank::{
    BankCoinFacts, BankDepositRequest, BankOperationIdentity, BankOperationOccurrence,
    BankOperationOutcome, BankResult,
};
use crate::durability::guild::{
    GUILD_GUARDS_PROPOSAL, GUILD_TABLES_PROPOSAL, GuildCommand, GuildDisbandRequest,
    GuildOperationIdentity, GuildOutcome, GuildRequest, GuildResignRequest, GuildResult,
};
use crate::durability::guild_bank::{
    GUILD_BANK_SCHEMA_PROPOSAL, GUILDHALL_LEDGER_SCHEMA_PROPOSAL, GuildBankCommand,
    GuildBankGuildId, GuildBankIdentity, GuildBankRequest,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator;
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

// Exact genuine Guild owner dependency; these hashes are a qualification overlay binding,
// not an installed migration or a second Guild schema owner.
fn assert_guild_owner_source() {
    assert_eq!(
        sha2::Sha256::digest(GUILD_TABLES_PROPOSAL.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "6d91fc5e5dbe597c72633f46dc328f30525a17498be1acbeabcc75829247343e"
    );
    assert_eq!(
        sha2::Sha256::digest(GUILD_GUARDS_PROPOSAL.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "387ba1c48521bb09a738acd6505084b65a64ec719dc4098171af5c9fb9cbb596"
    );
}
fn owner_id(kind: u8) -> [u8; 16] {
    let mut value = id(190);
    value[1] = kind;
    value
}
fn owner_request(
    n: u8,
    fence: CurrentCharacterItemFence,
    operation: GuildCommand,
) -> TestResult<GuildRequest> {
    let identity = bank_identity(n);
    Ok(GuildRequest {
        identity: GuildOperationIdentity {
            occurrence: identity.occurrence,
            transaction_id: identity.transaction_id,
            event_id: identity.guild_event_id,
            occurred_at_unix_ms: identity.occurred_at_unix_ms,
        },
        command: CommandRef::new(
            fence.game_session_id,
            CommandId::new(u64::from(n)).map_err(debug)?,
        ),
        guild_id: crate::domain::guild::GuildId::from_bytes(*guild().as_bytes()).map_err(debug)?,
        operation,
    })
}
fn successor_fence() -> TestResult<CurrentCharacterItemFence> {
    Ok(CurrentCharacterItemFence {
        character_id: crate::domain::CharacterId::from_bytes(id(95)).map_err(debug)?,
        game_session_id: GameSessionId::decode(&id(97)).map_err(debug)?,
        ..fence()?
    })
}
/// Privileged historical native Character/session DATA before Guild guards. The genuine
/// producer decoder/binding and independently reconciled Root/session/node predicates remain.
async fn historical_successor(h: &Harness) -> TestResult {
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid)")
        .bind(id(96).as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,'profile-1','ruleset-1','content-1','starter-1','Hall Successor')")
        .bind(id(95).as_slice()).bind(id(96).as_slice()).bind(id(WORLD).as_slice()).execute(&h.pool).await?;
    let operation = owner_id(1);
    let decision = owner_id(2);
    let raw = serde_json::to_vec(
        &serde_json::json!({"contract_version":2,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP",
        "issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":fixture::uuid_text(decision),
        "source_revision":"195","operation_id":fixture::uuid_text(operation),"operation":"INITIAL_CHARACTER_BOOTSTRAP",
        "account_id":fixture::uuid_text(id(96)),"target_world_id":fixture::uuid_text(id(WORLD)),"requested_name":"Hall Successor",
        "interpretation_context":{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"},
        "issued_at_source":"1","expires_at_source":"120","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}),
    )?;
    let intent = crate::character_bootstrap_intent::decode_producer_response(&raw, operation)
        .map_err(debug)?;
    sqlx::query("INSERT INTO game_character_operation_receipts(operation_id,command_binding,account_id,character_id,world_id,character_revision,event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,intent_source_revision,issued_at_source,expires_at_source) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',encode($7,'hex')::uuid,encode($8,'hex')::uuid,195,1,120)")
        .bind(operation.as_slice()).bind(intent.binding()).bind(id(96).as_slice()).bind(id(95).as_slice()).bind(id(WORLD).as_slice())
        .bind(owner_id(3).as_slice()).bind(owner_id(4).as_slice()).bind(decision.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_bootstrap_intent_floors(issuer_scope,source_revision,issuer_decision_id,intent_binding) SELECT 1,intent_source_revision,issuer_decision_id,command_binding FROM game_character_operation_receipts ORDER BY intent_source_revision DESC LIMIT 1 ON CONFLICT(issuer_scope) DO UPDATE SET source_revision=EXCLUDED.source_revision,issuer_decision_id=EXCLUDED.issuer_decision_id,intent_binding=EXCLUDED.intent_binding").execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,'',1)")
        .bind(id(95).as_slice()).bind(guild().as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_account_guild_leadership(account_id,guild_id,character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(id(96).as_slice()).bind(guild().as_bytes().as_slice()).bind(id(95).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,999999,1,1,1,1,$6,1)")
        .bind(id(97).as_slice()).bind(id(96).as_slice()).bind(id(95).as_slice()).bind(id(WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).bind(owner_id(5).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_durability_admission_account_guards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,'test',1,'account-current',1,0,'{}')")
        .bind(id(96).as_slice()).bind(id(95).as_slice()).bind(id(97).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_durability_admission_character_guards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,true,1,encode($4,'hex')::uuid,1,'test',1,'character-current',1,0,'{}')")
        .bind(id(95).as_slice()).bind(id(96).as_slice()).bind(id(WORLD).as_slice()).bind(id(97).as_slice()).execute(&h.pool).await?;
    Ok(())
}
async fn resign(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    actor: CurrentCharacterItemFence,
    successor: u8,
    revision: u64,
    command: u8,
) -> TestResult {
    let request = GuildResignRequest::new(
        owner_request(
            command,
            actor,
            GuildCommand::Resign(
                crate::domain::CharacterId::from_bytes(id(successor)).map_err(debug)?,
            ),
        )?,
        revision,
    )
    .map_err(debug)?;
    let premium =
        SpellPremiumCoordinator::new(std::sync::Arc::default(), None, None, Some(h.root.clone()));
    let outcome = h
        .root
        .commit_guild_resign(authority, &h.node, actor, request, &premium)
        .await
        .map_err(debug)?;
    assert!(matches!(outcome,GuildOutcome::Committed(receipt) if receipt.result==GuildResult::Ok));
    Ok(())
}
async fn install(h: &Harness) -> TestResult {
    assert_guild_owner_source();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SHOW server_version_num")
            .fetch_one(&h.pool)
            .await?,
        "170006"
    );
    for schema in [
        GUILD_TABLES_PROPOSAL,
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
    sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,vice_deficit_since,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Guildhall Fixture',2,9999999999999,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint,1,1)")
        .bind(guild().as_bytes().as_slice()).bind(id(WORLD).as_slice()).execute(&h.pool).await?;
    for (level, name) in [(1_i16, "Leader"), (2, "Vice Leader"), (3, "Member")] {
        sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,$2,$3)")
            .bind(guild().as_bytes().as_slice()).bind(level).bind(name).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,'',1)")
        .bind(id(CHARACTER).as_slice()).bind(guild().as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_account_guild_leadership(account_id,guild_id,character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(id(40).as_slice()).bind(guild().as_bytes().as_slice()).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    historical_successor(h).await?;
    h.pool.execute(sqlx::raw_sql(GUILD_GUARDS_PROPOSAL)).await?;
    // New genuine owner guards read their terminal journal on every Guild mutation.
    // Qualification read access preserves those guards and applies to inherited bodies too.
    h.pool
        .execute("GRANT SELECT ON game_guild_disband_steps TO oteryn_game_runtime")
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
            resign(&h, &a, fence()?, 95, 1, 140).await?;
            h.pool.execute("UPDATE game_house_auctions SET phase='RELEASING'").await?;
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
            resign(&h, &a, successor_fence()?, CHARACTER, 2, 141).await?;
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
            resign(&h, &a, fence()?, 95, 1, 150).await?;
            assert!(matches!(h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),step(31,None,Some(70))?).await,Err(GuildhallEscrowError::NotLeader)));
            resign(&h, &a, successor_fence()?, CHARACTER, 2, 151).await?;
            h.pool.execute("UPDATE game_house_auctions SET ends_at=clock_timestamp()-interval '1 second'").await?;
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
async fn conserved_gold(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT ((SELECT coalesce(sum(balance),0) FROM game_account_bank_balances)+(SELECT coalesce(sum(balance),0) FROM game_guild_bank_balances)+(SELECT coalesce(sum(escrow_guild_gold+escrow_account_gold),0) FROM game_guildhall_bids WHERE state='HELD'))::bigint")
        .fetch_one(&h.pool).await?)
}

#[test]
fn native_changed_leader_refuses_extra_account_funding_but_allows_guild_only_raise() -> TestResult {
    let admin = configured_admin().ok_or("native PostgreSQL17.6 required")?;
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guildhall_changed_leader").await?;
        let result:TestResult=async {
            install(&h).await?;
            let seal=h.recovery.seal_current().map_err(debug)?;
            let a=h.root.open_character_authority(&seal).await.map_err(debug)?;
            funded(&h,&a).await?;
            let unbound=h.pool.execute("UPDATE game_guilds SET revision=revision+1").await.expect_err("actual owner guard denies unbound mutation");
            assert!(matches!(unbound,sqlx::Error::Database(ref error) if error.code().as_deref()==Some("23514")));
            let original=h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),step(50,None,Some(70))?).await.map_err(debug)?;
            assert_eq!(original.parts,EscrowParts{guild:60,account:20});
            h.root.guild_bank_command(&a,&h.node,fence()?,GuildBankRequest{identity:bank_identity(51),guild_id:guild(),command:GuildBankCommand::Deposit,amount:10}).await.map_err(debug)?;
            assert_eq!(conserved_gold(&h).await?,100);
            resign(&h,&a,fence()?,95,1,152).await?;
            let successor=successor_fence()?;
            // Accepted Guild-first order can satisfy the entire raise without any new Account draw.
            let raised=h.root.guildhall_escrow_financial_step(&a,&h.node,Some(successor),step(52,Some(1),Some(80))?).await.map_err(debug)?;
            assert_eq!(raised.parts,EscrowParts{guild:70,account:20});
            let immutable:(String,String)=sqlx::query_as("SELECT funding_account_id::text,funding_character_id::text FROM game_guildhall_bids").fetch_one(&h.pool).await?;
            assert_eq!(immutable,(fixture::uuid_text(id(40)),fixture::uuid_text(id(CHARACTER))));
            let before=footprint(&h).await?;
            // Neither the old funding Account nor the new leader Account may silently fund
            // extra Account escrow while the architect's changed-leader policy is unresolved.
            for _ in 0..2 {
                assert!(matches!(h.root.guildhall_escrow_financial_step(&a,&h.node,Some(successor),step(53,Some(2),Some(85))?).await,Err(GuildhallEscrowError::FundingBindingChanged)));
                assert_eq!(footprint(&h).await?,before);assert_eq!(conserved_gold(&h).await?,100);
            }
            // Lowering refunds the fixed original Account first, despite the new current leader.
            let lower=step(54,Some(2),Some(65))?;
            let lowered=h.root.guildhall_escrow_financial_step(&a,&h.node,Some(successor),lower.clone()).await.map_err(debug)?;
            assert_eq!(lowered.parts,EscrowParts{guild:70,account:5});
            let before_replay=footprint(&h).await?;
            assert!(h.root.guildhall_escrow_financial_step(&a,&h.node,Some(successor),lower).await.map_err(debug)?.replayed);
            assert_eq!(footprint(&h).await?,before_replay);
            let original_account:i64=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid").bind(id(40).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(original_account,25);
            let successor_account:i64=sqlx::query_scalar("SELECT coalesce((SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid),0)").bind(id(96).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(successor_account,0);
            // House phase is explicit fixture DATA; this is financial release, not House disposition.
            h.pool.execute("UPDATE game_house_auctions SET phase='RELEASING'").await?;
            let release=step(55,Some(3),None)?;
            assert_eq!(h.root.guildhall_escrow_financial_step(&a,&h.node,None,release.clone()).await.map_err(debug)?.parts,EscrowParts::default());
            assert_eq!(conserved_gold(&h).await?,100);
            let after=footprint(&h).await?;assert_eq!(&after[4..],&[30,70]);
            assert!(h.root.guildhall_escrow_financial_step(&a,&h.node,None,release).await.map_err(debug)?.replayed);
            assert_eq!(footprint(&h).await?,after);
            Ok(())
        }.await;
        let cleanup=h.cleanup().await;result?;cleanup
    })
}

#[test]
fn native_real_guild_disband_start_closes_hall_bid_admission_without_fake_terminalization()
-> TestResult {
    let admin = configured_admin().ok_or("native PostgreSQL17.6 required")?;
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "guildhall_disband_start").await?;
        let result: TestResult = async {
            install(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let a = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            funded(&h, &a).await?;
            let request = GuildDisbandRequest::new(
                owner_request(153, fence()?, GuildCommand::Disband)?,
                "Guildhall Fixture",
            )
            .map_err(debug)?;
            let result = h
                .root
                .commit_guild_disband(&a, &h.node, fence()?, request)
                .await
                .map_err(debug)?;
            assert!(
                matches!(result,GuildOutcome::Committed(receipt) if receipt.result==GuildResult::Ok)
            );
            let before = footprint(&h).await?;
            assert!(matches!(
                h.root
                    .guildhall_escrow_financial_step(
                        &a,
                        &h.node,
                        Some(fence()?),
                        step(56, None, Some(70))?
                    )
                    .await,
                Err(GuildhallEscrowError::GuildIneligible)
            ));
            assert_eq!(footprint(&h).await?, before);
            assert_eq!(conserved_gold(&h).await?, 100);
            let state: i16 = sqlx::query_scalar("SELECT state FROM game_guilds")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(state, 3);
            assert_eq!(h.count("game_guild_members").await?, 2); // real pending start, not closure evidence.
            Ok(())
        }
        .await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}

// D1098 qualification uses the actual Guild owner's capability factory. These fixture-only
// grants permit locks/reads under runtime; no Guild writer/guard or C authority is replaced.
async fn disband_fixture_access(h: &Harness) -> TestResult {
    h.pool.execute(sqlx::raw_sql("GRANT SELECT ON game_guilds,game_guild_members,game_guild_invitations,game_account_guild_leadership,game_guild_job_checkpoints,game_guild_disband_steps TO oteryn_game_runtime; GRANT UPDATE(guild_id) ON game_guilds TO oteryn_game_runtime; GRANT UPDATE(character_id) ON game_guild_members,game_guild_invitations TO oteryn_game_runtime; GRANT UPDATE(account_id) ON game_account_guild_leadership TO oteryn_game_runtime;")).await?;
    Ok(())
}
fn disband_job(n: u8) -> TestResult<crate::durability::guild::GuildDisbandJobRequest> {
    Ok(crate::durability::guild::GuildDisbandJobRequest {
        world_id: crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,
        guild_id: crate::domain::guild::GuildId::from_bytes(*guild().as_bytes()).map_err(debug)?,
        hall_identity: step(n, Some(1), None)?.identity,
        bank_identity: bank_identity(n + 1),
    })
}
async fn start_disband(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    actor: CurrentCharacterItemFence,
    n: u8,
) -> TestResult {
    let request = GuildDisbandRequest::new(
        owner_request(n, actor, GuildCommand::Disband)?,
        "Guildhall Fixture",
    )
    .map_err(debug)?;
    let result = h
        .root
        .commit_guild_disband(authority, &h.node, actor, request)
        .await
        .map_err(debug)?;
    assert!(matches!(result,GuildOutcome::Committed(receipt) if receipt.result==GuildResult::Ok));
    Ok(())
}

#[test]
fn native_disband_bid_refund_real_guild_capability_rollback_replay_and_original_sources()
-> TestResult {
    let admin = configured_admin().ok_or("native PostgreSQL17.6 required")?;
    runtime()?.block_on(async move {
        let h = Harness::create(admin,"hall_disband_refund").await?;
        let result: TestResult = async {
            install(&h).await?;
            disband_fixture_access(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let a = h.root.open_character_authority(&seal).await.map_err(debug)?;
            funded(&h,&a).await?;
            h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),step(160,None,Some(70))?).await.map_err(debug)?;
            resign(&h,&a,fence()?,95,1,161).await?;
            start_disband(&h,&a,successor_fence()?,162).await?;
            let job = disband_job(163)?;
            let before = footprint(&h).await?;
            // A rollback includes both actual ledger writers and custody, with no minted grant.
            let mut tx = h.pool.begin().await?;
            tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
            let context = crate::durability::guild::lock_guild_disband_transaction(&mut tx,&h.root,&a,&h.node,&job).await.map_err(debug)?.ok_or("real disband context")?;
            assert_eq!(advance_guild_disband_release_in_transaction(&mut tx,&context).await.map_err(debug)?,GuildhallDisbandReleaseOutcome::WaitingForHouseDisposition);
            assert!(matches!(assert_guild_disband_released_in_transaction(&mut tx,&context).await,Err(GuildhallEscrowError::HouseDispositionUnavailable)));
            tx.rollback().await?;
            assert_eq!(footprint(&h).await?,before);
            // A capability from the rolled-back transaction cannot authorize a new physical TX.
            let mut tx = h.pool.begin().await?;
            assert!(matches!(advance_guild_disband_release_in_transaction(&mut tx,&context).await,Err(GuildhallEscrowError::NotCurrent)));
            tx.rollback().await?;
            assert_eq!(footprint(&h).await?,before);
            let mut tx = h.pool.begin().await?;
            tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
            let context = crate::durability::guild::lock_guild_disband_transaction(&mut tx,&h.root,&a,&h.node,&job).await.map_err(debug)?.ok_or("fresh disband context")?;
            assert_eq!(advance_guild_disband_release_in_transaction(&mut tx,&context).await.map_err(debug)?,GuildhallDisbandReleaseOutcome::WaitingForHouseDisposition);
            tx.commit().await?;
            assert_eq!(conserved_gold(&h).await?,100);
            let account: i64 = sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid").bind(id(40).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(account,40); // Refund original leader's Account, not the new leader.
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT balance FROM game_guild_bank_balances").fetch_one(&h.pool).await?,60);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_guildhall_bids WHERE state='HELD'").fetch_one(&h.pool).await?,0);
            // No fabricated RELEASING fixture context was required to refund owned custody.
            assert_eq!(sqlx::query_scalar::<_,String>("SELECT phase FROM game_house_auctions").fetch_one(&h.pool).await?,"OPEN");
            let released = footprint(&h).await?;
            let mut tx = h.pool.begin().await?;
            tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
            let context = crate::durability::guild::lock_guild_disband_transaction(&mut tx,&h.root,&a,&h.node,&job).await.map_err(debug)?.ok_or("replay context")?;
            assert_eq!(advance_guild_disband_release_in_transaction(&mut tx,&context).await.map_err(debug)?,GuildhallDisbandReleaseOutcome::WaitingForHouseDisposition);
            tx.commit().await?;
            assert_eq!(footprint(&h).await?,released);
            let mut conflicting = job.clone(); conflicting.hall_identity.transaction_id = id(169);
            let mut tx = h.pool.begin().await?;
            let context = crate::durability::guild::lock_guild_disband_transaction(&mut tx,&h.root,&a,&h.node,&conflicting).await.map_err(debug)?.ok_or("conflict context")?;
            assert!(matches!(advance_guild_disband_release_in_transaction(&mut tx,&context).await,Err(GuildhallEscrowError::ConflictingOccurrence)));
            tx.rollback().await?;
            assert_eq!(footprint(&h).await?,released);
            // Even no HELD and fixture AUCTION/VACANT are not genuine C closure authority.
            let mut tx = h.pool.begin().await?;
            let denied = sqlx::query("SELECT game_guildhall_assert_disband_released(encode($1,'hex')::uuid,encode($2,'hex')::uuid)").bind(guild().as_bytes().as_slice()).bind(id(WORLD).as_slice()).execute(&mut *tx).await.expect_err("no C final-release binding");
            assert_eq!(denied.as_database_error().and_then(|e|e.code()).as_deref(),Some("0A000"));
            tx.rollback().await?;
            assert_eq!(sqlx::query_scalar::<_,i16>("SELECT state FROM game_guilds").fetch_one(&h.pool).await?,3);
            assert_eq!(h.count("game_guild_members").await?,2);
            Ok(())
        }.await;
        let cleanup = h.cleanup().await; result?; cleanup
    })
}

#[test]
fn native_disband_bid_release_rejects_wrong_node_and_revoked_real_world_assignment() -> TestResult {
    let admin = configured_admin().ok_or("native PostgreSQL17.6 required")?;
    runtime()?.block_on(async move {
        let h = Harness::create(admin,"hall_disband_authority").await?;
        let result: TestResult = async {
            install(&h).await?; disband_fixture_access(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let a = h.root.open_character_authority(&seal).await.map_err(debug)?;
            funded(&h,&a).await?;
            h.root.guildhall_escrow_financial_step(&a,&h.node,Some(fence()?),step(170,None,Some(70))?).await.map_err(debug)?;
            start_disband(&h,&a,fence()?,171).await?;
            let job = disband_job(172)?;
            let before = footprint(&h).await?;
            let wrong = NodeIncarnationProof::new(h.node.fact(),crate::durability::runtime_scope_assignment::BootstrapSecret::from_bytes([99;32]));
            let mut tx = h.pool.begin().await?;
            assert!(matches!(crate::durability::guild::lock_guild_disband_transaction(&mut tx,&h.root,&a,&wrong,&job).await,Err(crate::durability::guild::GuildError::NotCurrent)));
            tx.rollback().await?;
            assert_eq!(footprint(&h).await?,before);
            let mut tx = h.pool.begin().await?;
            tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
            let context = crate::durability::guild::lock_guild_disband_transaction(&mut tx,&h.root,&a,&h.node,&job).await.map_err(debug)?.ok_or("current release context")?;
            assert_eq!(advance_guild_disband_release_in_transaction(&mut tx,&context).await.map_err(debug)?,GuildhallDisbandReleaseOutcome::WaitingForHouseDisposition);
            tx.commit().await?;
            let released = footprint(&h).await?;
            use crate::durability::runtime_scope_assignment::{RuntimeScopeAssignmentWriter,AssignmentRequest,AssignmentCommand,AssignmentPredecessor,OperationKey,ControlActor,AssignmentOutcome};
            let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(),"item-transfer-writer").await.map_err(debug)?;
            sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)").bind(id(WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
            let revoked = writer.submit(&AssignmentRequest{operation_key:OperationKey::from_bytes([45;32]),actor:ControlActor::new("oteryn_test_admin").map_err(debug)?,command:AssignmentCommand::Revoke{scope:fixture::scope()?,predecessor:AssignmentPredecessor{ownership_generation:1,source_revision:1,runtime_guard_publication_revision:Some(1)}}}).await.map_err(debug)?;
            assert!(matches!(revoked,AssignmentOutcome::Committed(_)));
            let mut tx = h.pool.begin().await?;
            assert!(matches!(crate::durability::guild::lock_guild_disband_transaction(&mut tx,&h.root,&a,&h.node,&job).await,Err(crate::durability::guild::GuildError::NotCurrent)));
            tx.rollback().await?;
            // Immutable release receipt does not recreate current World-job authority.
            assert_eq!(footprint(&h).await?,released);
            assert_eq!(conserved_gold(&h).await?,100);
            Ok(())
        }.await;
        let cleanup = h.cleanup().await; result?; cleanup
    })
}

#[test]
fn native_disband_zero_custody_bid_terminalizes_without_ledger_and_no_bid_never_proves_house_release()
-> TestResult {
    let admin = configured_admin().ok_or("native PostgreSQL17.6 required")?;
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "hall_disband_zero").await?;
        let result: TestResult = async {
            install(&h).await?;
            disband_fixture_access(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let a = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            funded(&h, &a).await?;
            h.pool
                .execute("UPDATE game_house_auctions SET rent_gold=0")
                .await?;
            h.root
                .guildhall_escrow_financial_step(
                    &a,
                    &h.node,
                    Some(fence()?),
                    step(180, None, Some(1))?,
                )
                .await
                .map_err(debug)?;
            h.root
                .guildhall_escrow_financial_step(
                    &a,
                    &h.node,
                    Some(fence()?),
                    step(181, Some(1), Some(0))?,
                )
                .await
                .map_err(debug)?;
            start_disband(&h, &a, fence()?, 182).await?;
            let before = footprint(&h).await?;
            let job = disband_job(183)?;
            let mut tx = h.pool.begin().await?;
            tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
            let context = crate::durability::guild::lock_guild_disband_transaction(
                &mut tx, &h.root, &a, &h.node, &job,
            )
            .await
            .map_err(debug)?
            .ok_or("zero held context")?;
            assert_eq!(
                advance_guild_disband_release_in_transaction(&mut tx, &context)
                    .await
                    .map_err(debug)?,
                GuildhallDisbandReleaseOutcome::WaitingForHouseDisposition
            );
            tx.commit().await?;
            let after = footprint(&h).await?;
            assert_eq!(after[0], before[0] + 1);
            assert_eq!(&after[1..], &before[1..]);
            let no_bid_job = disband_job(185)?;
            let mut tx = h.pool.begin().await?;
            tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
            let context = crate::durability::guild::lock_guild_disband_transaction(
                &mut tx,
                &h.root,
                &a,
                &h.node,
                &no_bid_job,
            )
            .await
            .map_err(debug)?
            .ok_or("no held context")?;
            assert_eq!(
                advance_guild_disband_release_in_transaction(&mut tx, &context)
                    .await
                    .map_err(debug)?,
                GuildhallDisbandReleaseOutcome::WaitingForHouseDisposition
            );
            tx.commit().await?;
            assert_eq!(footprint(&h).await?, after);
            assert_eq!(conserved_gold(&h).await?, 100);
            Ok(())
        }
        .await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}
