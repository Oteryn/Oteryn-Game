//! TRADE-PLAYER-1 writer qualification only. The sole auto mount runs on the
//! game-server library PG lane; runtime/dispatch activation awaits session ingress.
use crate::domain::CharacterId;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome,
    ItemWeightLookup, ItemWeightRefused,
};
use crate::durability::player_trade::{
    PlayerTradeContent, PlayerTradeRequest, TradeParticipant, TradeReceipt,
};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use crate::item_transfer_postgres_cases::{
    BACKPACK, CHANNEL, CHARACTER, COIN, Harness, SESSION, TestResult, WORLD, backpack_facts,
    configured_admin, debug, facts, fence, id, join_two, runtime, to_backpack, to_slot,
};
use oteryn_gameplay_economy::player_trade::{Refusal, TradeFlag};

const SECOND_ACCOUNT: u8 = 60;
const SECOND_CHARACTER: u8 = 61;
const SECOND_SESSION: u8 = 62;
const DIGEST: [u8; 32] = [1; 32];

fn command_of(session: u8, value: u64) -> TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&id(session)).map_err(debug)?,
        CommandId::new(value).map_err(debug)?,
    ))
}
fn fences() -> TestResult<[CurrentCharacterItemFence; 2]> {
    let a = fence()?;
    Ok([
        a,
        CurrentCharacterItemFence {
            character_id: CharacterId::from_bytes(id(SECOND_CHARACTER)).map_err(debug)?,
            game_session_id: GameSessionId::decode(&id(SECOND_SESSION)).map_err(debug)?,
            ..a
        },
    ])
}

#[derive(Clone, Copy)]
struct Content {
    flag: TradeFlag,
    unit: Option<u32>,
    room: u32,
}
const CONTENT: Content = Content {
    flag: TradeFlag::Unknown,
    unit: Some(1),
    room: 20,
};
impl ItemWeightLookup for Content {
    fn generation_digest(&self) -> [u8; 32] {
        [0xd1; 32]
    }
    fn weight_of(&self, _: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        self.unit.ok_or(ItemWeightRefused::Unknown)
    }
}
impl ItemLoopContent for Content {
    fn loop_content_digest(&self) -> [u8; 32] {
        DIGEST
    }
    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        let mut result = if definition.production_key == BACKPACK {
            backpack_facts()
        } else {
            facts(
                COIN,
                ItemStackClass::Stackable {
                    proven_maximum: Some(100),
                },
            )
        };
        if result.definition != *definition {
            return None;
        }
        result.weight = self.unit;
        if result.container_capacity.is_some() {
            result.container_capacity = Some(self.room);
        }
        Some(result)
    }
}
impl PlayerTradeContent for Content {
    fn player_trade_flags(&self, _: &TypedDefinitionRef) -> Option<(TradeFlag, bool)> {
        Some((self.flag, true))
    }
}

// Reuse the canonical two-Character bootstrap shape of corpse_transfer's
// seed_second_character, with no additional mounted PG target or copied owner.
fn second_bootstrap_binding() -> TestResult<Vec<u8>> {
    let mut binding = vec![2];
    binding.extend_from_slice(&id(63));
    binding.extend_from_slice(&2_i64.to_be_bytes());
    binding.extend_from_slice(&id(64));
    binding.extend_from_slice(&id(SECOND_ACCOUNT));
    binding.extend_from_slice(&id(WORLD));
    binding.extend_from_slice(&1_i64.to_be_bytes());
    binding.extend_from_slice(&120_i64.to_be_bytes());
    for value in ["profile-1", "ruleset-1", "content-1", "starter-1"] {
        binding.extend_from_slice(&u16::try_from(value.len())?.to_be_bytes());
        binding.extend_from_slice(value.as_bytes());
    }
    // Contract version 2 binds the requested name last (CHAR-NAME-1).
    binding.extend_from_slice(&11_u16.to_be_bytes());
    binding.extend_from_slice(b"Second Hero");
    Ok(binding)
}

/// A second player in the same World and Channel: Character 61 (account 60)
/// with progression state, an active GameSession 62 and its admission guards; the
/// runtime-scope assignment and readiness are the harness's, shared.
async fn seed_second_character(pool: &sqlx::PgPool) -> TestResult {
    sqlx::query("INSERT INTO game_character_account_guards VALUES (encode($1,'hex')::uuid)")
        .bind(id(SECOND_ACCOUNT).as_slice())
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_roots VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          1,1,'profile-1','ruleset-1','content-1','starter-1','Second Hero')",
    )
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(SECOND_ACCOUNT).as_slice())
    .bind(id(WORLD).as_slice())
    .execute(pool)
    .await?;
    // Match the first Character's native fixture progression. Incoming capacity
    // reads this row after the Character root; a missing row is invalid state.
    sqlx::query(
        "INSERT INTO game_character_progression_state VALUES \
         (encode($1,'hex')::uuid,1,50,1000,'profile-1','ruleset-1','content-1',\
          'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
    )
    .bind(id(SECOND_CHARACTER).as_slice())
    .execute(pool)
    .await?;
    // `open_character_authority` verifies Character integrity: every root needs
    // its bootstrap receipt, and the single intent floor must name the receipt
    // with the highest source revision (here 2, advancing the harness's 1).
    let binding = second_bootstrap_binding()?;
    sqlx::query(
        "INSERT INTO game_character_operation_receipts(\
           operation_id,command_binding,account_id,character_id,world_id,character_revision,\
           event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,\
           intent_source_revision,issued_at_source,expires_at_source) \
         VALUES (encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,\
           encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',\
           encode($7,'hex')::uuid,encode($8,'hex')::uuid,2,1,120)",
    )
    .bind(id(64).as_slice())
    .bind(&binding)
    .bind(id(SECOND_ACCOUNT).as_slice())
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(WORLD).as_slice())
    .bind(id(65).as_slice())
    .bind(id(66).as_slice())
    .bind(id(63).as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE game_character_bootstrap_intent_floors \
            SET source_revision = 2, issuer_decision_id = encode($1,'hex')::uuid, \
                intent_binding = $2 \
          WHERE issuer_scope = 1",
    )
    .bind(id(63).as_slice())
    .bind(&binding)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_reconnect_sessions(\
           game_session_id,account_id,character_id,world_id,runtime_scope_kind,\
           runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,\
           original_grace_deadline,predecessor_generation,character_lease_generation,\
           scope_ownership_generation,current_generation,current_transport_ref,session_state) \
         VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
           encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,\
           1,999999,1,1,1,1,$6,1)",
    )
    .bind(id(SECOND_SESSION).as_slice())
    .bind(id(SECOND_ACCOUNT).as_slice())
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(WORLD).as_slice())
    .bind(id(CHANNEL).as_slice())
    .bind([10_u8; 16].as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_admission_account_guards VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          1,'test',1,'account-current',1,0,'{}')",
    )
    .bind(id(SECOND_ACCOUNT).as_slice())
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(SECOND_SESSION).as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_admission_character_guards VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          true,1,encode($4,'hex')::uuid,1,'test',1,'character-current',1,0,'{}')",
    )
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(SECOND_ACCOUNT).as_slice())
    .bind(id(WORLD).as_slice())
    .bind(id(SECOND_SESSION).as_slice())
    .execute(pool)
    .await?;
    Ok(())
}

async fn activate(h: &Harness) -> TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).bind(DIGEST.as_slice()).bind([2_u8;32].as_slice()).bind([3_u8;32].as_slice())
        .execute(&h.pool).await?;
    Ok(())
}
async fn inventory(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<[[u8; 16]; 2]> {
    let mut result = [[0; 16]; 2];
    for (index, f) in fences()?.into_iter().enumerate() {
        let session = if index == 0 { SESSION } else { SECOND_SESSION };
        let backpack = h.mint(authority, BACKPACK, 1).await?;
        assert!(matches!(
            h.transfer(
                authority,
                f,
                to_slot(command_of(session, 1)?, backpack, backpack_facts())
            )
            .await
            .map_err(debug)?,
            ItemTransferOutcome::Committed(_)
        ));
        let item = h
            .mint(authority, COIN, if index == 0 { 7 } else { 13 })
            .await?;
        assert!(matches!(
            h.transfer(
                authority,
                f,
                to_backpack(
                    command_of(session, 2)?,
                    item,
                    facts(
                        COIN,
                        ItemStackClass::Stackable {
                            proven_maximum: Some(100)
                        }
                    )
                )
            )
            .await
            .map_err(debug)?,
            ItemTransferOutcome::Committed(_)
        ));
        result[index] = item;
    }
    Ok(result)
}
async fn request(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    items: [[u8; 16]; 2],
    tag: u8,
) -> TestResult<PlayerTradeRequest> {
    let fs = fences()?;
    let commands = [
        command_of(SESSION, u64::MAX)?,
        command_of(SECOND_SESSION, u64::MAX - 1)?,
    ];
    let [a, b] = h
        .root
        .observe_player_trade(authority, &h.node, fs, commands, items, DIGEST)
        .await?
        .map_err(debug)?;
    Ok(PlayerTradeRequest {
        pair: command_of(SECOND_SESSION, 90)?,
        occurrence_id: id(tag),
        transaction_id: id(tag + 1),
        event_id: id(tag + 2),
        occurred_at_unix_ms: 1_800_000_000_000,
        participants: [
            TradeParticipant {
                accepted: fs[0],
                command: commands[0],
                leaf: a,
            },
            TradeParticipant {
                accepted: fs[1],
                command: commands[1],
                leaf: b,
            },
        ],
        content_digest: DIGEST,
        weight_digest: CONTENT.generation_digest(),
        node_id: *h.node.fact().node_id().as_bytes(),
        node_registration_revision: h.node.fact().registration_revision(),
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    })
}
async fn footprint(h: &Harness) -> TestResult<serde_json::Value> {
    let value=sqlx::query_scalar("SELECT jsonb_build_object( \
        'items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i), \
        'entries',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_container_entries e), \
        'trades',(SELECT count(*) FROM game_player_trades), 'participants',(SELECT count(*) FROM game_player_trade_participants), \
        'receipts',(SELECT count(*) FROM game_player_trade_receipts), 'events',(SELECT count(*) FROM game_item_audit_outbox))")
        .fetch_one(&h.pool).await?;
    Ok(value)
}

#[test]
fn player_trade_postgres_atomic_replay_refusals_and_restart() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h=Harness::create(admin,"player_trade_atomic").await?;
        seed_second_character(&h.pool).await?;
        activate(&h).await?;
        let seal=h.recovery.seal_current().map_err(debug)?;
        let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        let items=inventory(&h,&authority).await?;
        let req=request(&h,&authority,items,180).await?;
        let permit=SpellLanePermit::of_fresh_scope(fence()?.runtime_scope).await?;
        let before=footprint(&h).await?;
        assert_eq!(h.root.reconcile_player_trade(&authority,req.clone()).await?,Ok(TradeReceipt::Absent));
        for content in [Content {flag:TradeFlag::Known(false),..CONTENT},Content {flag:TradeFlag::Ambiguous,..CONTENT},
            Content {unit:None,..CONTENT},Content {unit:Some(156_000),..CONTENT},Content {room:1,..CONTENT}] {
            let result=h.root.exchange_player_trade(&permit,&authority,&h.node,fences()?,&content,req.clone()).await?;
            assert!(result.is_err(),"{result:?}");
            assert_eq!(footprint(&h).await?,before);
        }
        // Mutate one captured before-value at a time; the current DB remains valid.
        for side in [0,1] {
            for (section,field,value) in [
                ("item","quantity",serde_json::json!(99)),
                ("key","key_number",serde_json::json!(7)),
                ("item","definition_revision_ref",serde_json::json!("stale")),

                ("item","state_revision",serde_json::json!("99")),
                ("item","lifecycle",serde_json::json!(2)),
                ("entry","placement_ordinal",serde_json::json!("99")),
                ("entry","placed_transaction_id",serde_json::json!(crate::item_transfer_postgres_cases::uuid_text(id(199)))),
            ] {
                let mut changed=req.clone();
                let mut value_tree=serde_json::to_value(&changed.participants[side].leaf)?;
                value_tree[section][field]=value;
                changed.participants[side].leaf=serde_json::from_value(value_tree)?;
                assert_eq!(h.root.exchange_player_trade(&permit,&authority,&h.node,fences()?,&CONTENT,changed).await?,Err(Refusal::Stale));
                assert_eq!(footprint(&h).await?,before);
            }
        }
        // Every fence axis changes independently, with the remaining state valid.
        for side in [0,1] {
            for axis in [0,1,2,3,4] {
                let mut current=fences()?;
                match axis {
                    0=>current[side].character_lease_generation+=1,
                    1=>current[side].connection_generation=crate::foundation::ConnectionGeneration::new(2).map_err(debug)?,
                    2=>current[side].scope_ownership_generation=crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?,
                    3=>current[side].game_session_id=GameSessionId::decode(&id(99)).map_err(debug)?,
                    _=>current[side].runtime_scope=crate::foundation::RuntimeScopeRefV1::channel(
                        crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,crate::foundation::ChannelId::decode(&id(99)).map_err(debug)?),
                }
                assert!(h.root.exchange_player_trade(&permit,&authority,&h.node,current,&CONTENT,req.clone()).await?.is_err());
                assert_eq!(footprint(&h).await?,before);
            }
        }
        let mut foreign=req.clone();foreign.node_registration_revision+=1;
        assert_eq!(h.root.exchange_player_trade(&permit,&authority,&h.node,fences()?,&CONTENT,foreign).await?,Err(Refusal::Rejected));
        // Runtime may append complete outcomes, but cannot alter existing ones.
        for table in ["game_player_trades","game_player_trade_participants","game_player_trade_receipts"] {
            let granted: bool=sqlx::query_scalar("SELECT has_table_privilege('oteryn_game_runtime',$1,'SELECT') AND has_table_privilege('oteryn_game_runtime',$1,'INSERT') AND NOT has_table_privilege('oteryn_game_runtime',$1,'UPDATE,DELETE,TRUNCATE')")
                .bind(table).fetch_one(&h.pool).await?;
            assert!(granted);
        }
        let mut incomplete=h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *incomplete).await?;
        sqlx::query("INSERT INTO game_player_trades(occurrence_id,transaction_id,event_id,world_id,channel_id,intent,occurred_at,envelope_sha256,pair_game_session_id,pair_command_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,1,$7,encode($8,'hex')::uuid,1)")
            .bind(id(190).as_slice()).bind(id(191).as_slice()).bind(id(192).as_slice()).bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice())
            .bind([1_u8].as_slice()).bind([0_u8;32].as_slice()).bind(id(SESSION).as_slice()).execute(&mut *incomplete).await?;
        let incomplete_result=incomplete.commit().await;
        assert!(matches!(incomplete_result,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("23514")));
        assert_eq!(footprint(&h).await?,before);
        // Force a failure at the second destination INSERT, after both DELETEs and the
        // first reciprocal insertion. The whole value/audit transaction must roll back.
        sqlx::query("CREATE FUNCTION player_trade_test_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.character_id=encode(decode('2902030405067008800a0b0c0d0e0f29','hex'),'hex')::uuid THEN RAISE EXCEPTION 'injected second half failure' USING ERRCODE='23514'; END IF; RETURN NEW; END $$")
            .execute(&h.pool).await?;
        sqlx::query("CREATE TRIGGER player_trade_test_fail BEFORE INSERT ON game_item_container_entries FOR EACH ROW EXECUTE FUNCTION player_trade_test_fail()")
            .execute(&h.pool).await?;
        assert!(h.root.exchange_player_trade(&permit,&authority,&h.node,fences()?,&CONTENT,req.clone()).await.is_err());
        assert_eq!(footprint(&h).await?,before);
        assert_eq!(h.root.reconcile_player_trade(&authority,req.clone()).await?,Ok(TradeReceipt::Absent));
        sqlx::query("DROP TRIGGER player_trade_test_fail ON game_item_container_entries").execute(&h.pool).await?;
        // A first accepter can resume the same GameSession on a newer connection.
        // Its lease/scope/session remain pinned; the triggering side cannot advance.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=2 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(id(SESSION).as_slice()).execute(&h.pool).await?;
        let mut current=fences()?;
        current[0].connection_generation=crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
        // Two identical in-flight attempts serialize on the occurrence; exactly one swaps.
        let concurrent=crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(concurrent.maintain_ready_once().await?);
        let concurrent_authority=concurrent.open_character_authority(&seal).await.map_err(debug)?;
        let (first,second)=join_two(
            h.root.exchange_player_trade(&permit,&authority,&h.node,current,&CONTENT,req.clone()),
            concurrent.exchange_player_trade(&permit,&concurrent_authority,&h.node,current,&CONTENT,req.clone()),
        ).await;
        let outcomes=[first?,second?];
        assert!(outcomes.contains(&Ok(TradeReceipt::Committed)),"{outcomes:?}");
        assert!(outcomes.contains(&Ok(TradeReceipt::AlreadyCommitted)),"{outcomes:?}");
        let after=footprint(&h).await?;
        let old_items=before["items"].as_array().ok_or("missing before items")?;
        let new_items=after["items"].as_array().ok_or("missing after items")?;
        for (old,new) in old_items.iter().zip(new_items) {
            let mut old=old.clone();let mut new=new.clone();
            old.as_object_mut().ok_or("before object")?.remove("last_transaction_id");
            new.as_object_mut().ok_or("after object")?.remove("last_transaction_id");
            assert_eq!(old,new,"all item state, including revision, survives the swap");
        }
        assert_eq!(after["trades"],1);assert_eq!(after["participants"],2);assert_eq!(after["receipts"],2);
        for (index,item) in items.iter().enumerate() {
            let owner:String=sqlx::query_scalar("SELECT character_id::text FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(item.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(owner,crate::item_transfer_postgres_cases::uuid_text(id(if index==0 {SECOND_CHARACTER} else {CHARACTER})));
            assert_eq!(h.item_state(*item).await?,(if index==0 {7} else {13},1));
        }
        // A closed READY pair cannot be adopted by fresh accept commands/occurrence.
        let mut reused=req.clone();
        reused.occurrence_id=id(193);reused.transaction_id=id(194);reused.event_id=id(195);
        for (index,p) in reused.participants.iter_mut().enumerate() {
            p.accepted=current[index];
            p.command=command_of(if index==0 {SESSION} else {SECOND_SESSION},100+u64::try_from(index)?)?;
        }
        assert_eq!(h.root.exchange_player_trade(&permit,&authority,&h.node,current,&CONTENT,reused).await?,Err(Refusal::Rejected));
        assert_eq!(footprint(&h).await?,after);
        let mut conflict=req.clone();conflict.sim_revision="different".into();
        assert_eq!(h.root.reconcile_player_trade(&authority,conflict).await?,Err(Refusal::Rejected));
        // Same-head historical replay bypasses dead sessions and changed Content.
        let mut stale=fences()?;stale[0].character_lease_generation+=1;
        assert_eq!(h.root.exchange_player_trade(&permit,&authority,&h.node,stale,&Content {flag:TradeFlag::Known(false),..CONTENT},req.clone()).await?,Ok(TradeReceipt::AlreadyCommitted));
        assert_eq!(footprint(&h).await?,after);
        drop(concurrent_authority);drop(concurrent);
        drop(authority);
        let restarted=crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let recovered=restarted.open_character_authority(&seal).await.map_err(debug)?;
        assert_eq!(restarted.reconcile_player_trade(&recovered,req).await?,Ok(TradeReceipt::AlreadyCommitted));
        assert_eq!(footprint(&h).await?,after);
        drop(recovered);drop(restarted);drop(permit);drop(seal);
        h.cleanup().await
    })
}
