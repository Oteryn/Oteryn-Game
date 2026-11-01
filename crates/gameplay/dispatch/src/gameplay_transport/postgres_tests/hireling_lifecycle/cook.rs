//! Cook and imbuement packages enter through command 7 and the existing economic owners.
use super::{Harness, TestResult, debug, id};
use crate::foundation::{ExactActorRef, MessageType, decode_wire_envelope};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession, ConnectionEnd, UseCommand,
    IDLE_LIVENESS, serve_admitted};
use oteryn_protocol_oteryn::{ClientCommandValue, CommandStatus, decode_command_result,
    encode_client_command};
use oteryn_protocol_oteryn::npc_service as wire;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) async fn scenario(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession, actor: ExactActorRef, hireling: [u8; 16]) -> TestResult {
    super::super::item_look::seed_main_backpack(h, &[("oteryn:item.tibia.i3043", 20)]).await?;
    let npc = prepare_npc(owner, admitted, actor, hireling).await?;
    let (mut server,stream) = tokio::io::duplex(1 << 20);
    let mut client = Client::new(stream,admitted);
    let client_run = async {
    client.talk(npc, "hi").await?;
    let missing = client.talk(npc, "food").await?;
    assert!(missing.lines.iter().any(|line| line.contains("do not have mastery")));
    client.talk(npc, "goods").await?;
    let free = client.talk_window(npc, "various").await?;
    assert!(free.window.as_ref().is_some_and(|window| !window.offers.is_empty()),
        "the unskilled Hireling keeps the donor's basic various service");
    let locked = client.talk(npc, "buy loot").await?;
    assert!(locked.lines.iter().any(|line| line.contains("do not have mastery")));
    assert!(locked.window.is_none(), "a missing Trader entitlement opens no expanded page");

    entitlement(h, id(41), 1002, true).await?;
    client.talk(npc, "hi").await?;
    client.talk(npc, "food").await?;
    // Choose a real next command whose deterministic source draw opens the skill menu.
    // The client still sends no roll. Filler requests are valid, non-purchasing menu talks.
    for _ in 0..128 {
        if draw(owner,admitted.game_session_id,client.next_command,hireling)? == 5 { break; }
        client.talk(npc, "menu").await?;
    }
    assert_eq!(draw(owner,admitted.game_session_id,client.next_command,hireling)?, 5, "bounded deterministic surprise seed");
    let menu = client.talk(npc, "surprise").await?;
    assert!(menu.lines.iter().any(|line| line.contains("distance skill")),
        "native surprise must open the skill menu: {:?}", menu.lines);
    let distance_command = client.next_command;
    let meal = client.talk(npc, "distance").await?;
    assert!(meal.lines.iter().any(|line| line.contains("delivered to your Inbox")));
    let dish: (i64, String, String) = sqlx::query_as("SELECT r.item_id,r.price_gold::text,i.definition_production_key \
        FROM game_hireling_food_receipts r JOIN game_item_instances i ON i.item_instance_id=r.food_item_instance_id")
        .fetch_one(&h.pool).await?;
    assert_eq!(dish, (29409, "15000".into(), "oteryn:item.tibia.i29409".into()),
        "ANSWER Q10 selects the actual distance dish, not Crystal's toolbox typo");
    assert_eq!(count(h, "SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i35173'").await?, 0);
    assert_eq!(coins(h).await?, 185_000);
    assert_item_only_revision_one(h, 1).await?;
    client.talk_command(npc, "distance", distance_command).await?;
    assert_resume(owner, admitted, npc, "distance", distance_command).await?;
    assert_eq!(count(h, "SELECT count(*) FROM game_hireling_food_receipts").await?, 1);
    assert_eq!(coins(h).await?, 185_000, "replay cannot charge a second meal");

    for text in ["food", "specific"] { client.talk(npc, text).await?; }
    client.talk(npc, "carrot pie").await?;
    assert_eq!(count(h, "SELECT count(*) FROM game_hireling_food_receipts WHERE item_id=29409 AND price_gold=90000").await?, 1);
    assert_eq!(coins(h).await?, 95_000, "the specific dish uses its distinct donor price");
    assert_eq!(count(h, "SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=472 AND house_key='oteryn:content.house.admiral_s_avenue_1'").await?, 2);
    assert_eq!(count(h, "SELECT count(*) FROM game_item_audit_outbox a JOIN game_item_fee_burns f USING(transaction_id) WHERE f.cause_kind=472").await?, 2);
    assert_eq!(count(h, "SELECT count(*) FROM game_item_character_inbox_locations b JOIN game_hireling_food_receipts r ON r.food_item_instance_id=b.item_instance_id").await?, 2);
    assert_item_only_revision_one(h, 2).await?;

    // Packages are the donor's free service, independent of the four paid skill unlocks.
    entitlement(h, id(41), 1002, false).await?;
    for text in ["imbuement packages", "bash"] { client.talk(npc, text).await?; }
    let package_command = client.next_command;
    let package = client.talk(npc, "yes").await?;
    assert!(package.lines.iter().any(|line| line.contains("package is in your backpack")));
    assert_eq!(coins(h).await?, 88_750);
    let contents: Vec<(String, i64)> = sqlx::query_as("SELECT i.definition_production_key,i.quantity \
        FROM game_hireling_package_receipts p JOIN game_item_nested_entries n ON n.parent_item_instance_id=p.present_item_instance_id \
        JOIN game_item_instances i ON i.item_instance_id=n.item_instance_id ORDER BY n.placement_ordinal")
        .fetch_all(&h.pool).await?;
    assert_eq!(contents, vec![("oteryn:item.tibia.i9657".into(),20),
        ("oteryn:item.tibia.i22189".into(),15),("oteryn:item.tibia.i10405".into(),10)]);
    assert_eq!(count(h, "SELECT count(*) FROM game_item_container_entries e JOIN game_hireling_package_receipts p ON p.present_item_instance_id=e.item_instance_id JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id WHERE i.definition_production_key='oteryn:item.tibia.i2856'").await?, 1);
    let envelope: Vec<u8> = sqlx::query_scalar("SELECT a.envelope FROM game_item_audit_outbox a JOIN game_hireling_package_receipts p USING(transaction_id)")
        .fetch_one(&h.pool).await?;
    let (_, audit) = crate::durability::npc_trade_audit::decode_npc_trade_envelope(&envelope).map_err(debug)?;
    assert_eq!(audit.house_key.as_deref(), Some(super::HOUSE));
    let package = audit.hireling_package.ok_or("same native trade audit contains the present")?;
    assert_eq!(package.name, "bash");
    assert_eq!(package.children.len(), 3);
    client.talk_command(npc, "yes", package_command).await?;
    assert_resume(owner, admitted, npc, "yes", package_command).await?;
    assert_eq!(coins(h).await?, 88_750);
    assert_eq!(count(h, "SELECT count(*) FROM game_hireling_package_receipts").await?, 1);
    assert_eq!(count(h, "SELECT count(*) FROM game_hireling_package_lines").await?, 3);
    assert_eq!(count(h, "SELECT count(*) FROM game_item_audit_outbox a JOIN game_hireling_package_receipts p USING(transaction_id)").await?, 1);
    client.close().await?;
    Ok(())
    };
    drive(owner,admitted,&mut server,client_run).await?;
    Ok(())
}

pub(super) async fn bank_only_scenario(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession, actor: ExactActorRef, hireling: [u8;16]) -> TestResult {
    // Match GOLD-FEE-ACT-2 and gold_fee_bank_postgres_cases::Harness::activate.
    // Bank fallback is deliberately unavailable under the original V1 tuple.
    // This isolated scenario activates V2 through its existing migration-owner
    // boundary; the runtime fee owner still selects and enforces the tuple.
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES (1, now())")
        .execute(&h.pool).await?;
    assert_eq!(count(h,"SELECT count(*) FROM game_type2_audit_activation WHERE id=1").await?,1);
    // Retained historical Bank prefix, matching the native gold-fee PostgreSQL fixtures.
    // No current backpack or physical coins are created for this customer.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,sha256('seed'),encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,1,30000,0,1,sha256('seed'),0,encode($8,'hex')::uuid,encode($9,'hex')::uuid,encode($10,'hex')::uuid)")
        .bind(id(230).as_slice()).bind(id(231).as_slice()).bind(id(232).as_slice())
        .bind(id(41).as_slice()).bind(id(40).as_slice()).bind(id(42).as_slice())
        .bind(admitted.channel_id.as_bytes().as_slice()).bind(id(233).as_slice())
        .bind(id(234).as_slice()).bind(id(235).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,30000,0,30000,encode($5,'hex')::uuid)")
        .bind(id(236).as_slice()).bind(id(231).as_slice()).bind(id(40).as_slice())
        .bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,30000,encode($3,'hex')::uuid)")
        .bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(236).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    entitlement(h,id(41),1002,true).await?;
    assert_eq!(count(h,"SELECT count(*) FROM game_item_container_slots").await?,0);
    let npc = prepare_npc(owner,admitted,actor,hireling).await?;
    let (mut server,stream) = tokio::io::duplex(1 << 20);
    let mut client = Client::new(stream,admitted);
    let client_run = async {
    for text in ["hi","food"] { client.talk(npc, text).await?; }
    for _ in 0..128 {
        if draw(owner,admitted.game_session_id,client.next_command,hireling)?==5 { break; }
        client.talk(npc, "menu").await?;
    }
    assert_eq!(draw(owner,admitted.game_session_id,client.next_command,hireling)?,5);
    let menu = client.talk(npc, "surprise").await?;
    assert!(menu.lines.iter().any(|line|line.contains("distance skill")),
        "bank-only native surprise must open the skill menu: {:?}", menu.lines);
    let command = client.next_command;
    let bought = client.talk(npc, "distance").await?;
    assert!(bought.lines.iter().any(|line|line.contains("delivered to your Inbox")),
        "bank-only Cook reply: {:?}", bought.lines);
    assert_eq!(count(h,"SELECT count(*) FROM game_hireling_food_receipts WHERE item_id=29409 AND price_gold=15000").await?,1);
    assert_eq!(count(h,"SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i35173'").await?,0);
    assert_eq!(count(h,"SELECT balance FROM game_account_bank_balances").await?,15_000);
    assert_eq!(count(h,"SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=472 AND bank_debit_gold_units=15000 AND burned_gold_units=0 AND change_gold_units=0 AND line_count=0 AND backpack_item_instance_id IS NULL").await?,1);
    assert_eq!(count(h,"SELECT count(*) FROM game_item_audit_outbox a JOIN game_item_fee_burns f USING(transaction_id) WHERE f.cause_kind=472 AND a.schema_revision=2 AND a.retention_profile_id='DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2'").await?,1,
        "the native bank debit is carried only by the activated V2 audit tuple");
    assert_eq!(count(h,"SELECT count(*) FROM game_item_fee_burn_lines").await?,0);
    assert_eq!(count(h,"SELECT count(*) FROM game_item_container_slots").await?,0);
    assert_eq!(count(h,"SELECT count(*) FROM game_item_character_inbox_locations b JOIN game_hireling_food_receipts r ON r.food_item_instance_id=b.item_instance_id").await?,1);
    let envelope: Vec<u8> = sqlx::query_scalar("SELECT a.envelope FROM game_item_audit_outbox a JOIN game_hireling_food_receipts r ON r.fee_transaction_id=a.transaction_id")
        .fetch_one(&h.pool).await?;
    let (_,fee) = crate::durability::item_fee_burn_audit::decode_fee_burn_envelope(&envelope).map_err(debug)?;
    assert_eq!(fee.house_key.as_deref(),Some(super::HOUSE));
    assert!(fee.bank_debit.is_some());
    assert!(fee.lines.is_empty());
    client.talk_command(npc, "distance",command).await?;
    assert_resume(owner,admitted,npc,"distance",command).await?;
    assert_eq!(count(h,"SELECT balance FROM game_account_bank_balances").await?,15_000);
    assert_eq!(count(h,"SELECT count(*) FROM game_hireling_food_receipts").await?,1);
    assert_eq!(count(h,"SELECT count(*) FROM game_account_bank_entries").await?,2);
    assert_eq!(count(h,"SELECT count(*) FROM game_item_audit_outbox a JOIN game_item_fee_burns f USING(transaction_id) WHERE f.cause_kind=472").await?,1);
    assert_item_only_revision_one(h, 1).await?;
    client.close().await?;
    Ok(())
    };
    drive(owner,admitted,&mut server,client_run).await?;
    Ok(())
}

/// Cook is an item-only native Inbox/fee composition, including a newly created
/// Character at revision 1. It must neither invent a Character revision nor
/// expose the native Inbox owner's private receipts to make its guard succeed.
async fn assert_item_only_revision_one(h: &Harness, expected_receipts: usize) -> TestResult {
    let current: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(current, "1", "a food delivery does not advance the Character root");
    let rows: Vec<(String, String, Vec<u8>)> = sqlx::query_as("SELECT r.committed_character_revision::text,f.committed_character_revision::text,a.envelope \
        FROM game_hireling_food_receipts r JOIN game_item_fee_burns f ON f.transaction_id=r.fee_transaction_id \
        JOIN game_item_audit_outbox a ON a.transaction_id=f.transaction_id \
        WHERE r.character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(rows.len(), expected_receipts);
    for (food_revision, fee_revision, envelope) in rows {
        assert_eq!((food_revision.as_str(), fee_revision.as_str()), ("1", "1"));
        let (_, fee) = crate::durability::item_fee_burn_audit::decode_fee_burn_envelope(&envelope).map_err(debug)?;
        assert_eq!(fee.committed_character_revision, 1,
            "the native fee audit retains the same actual item-only revision");
    }
    let private: (bool, bool) = sqlx::query_as("SELECT \
        has_table_privilege('oteryn_game_runtime','game_reward_claim_inbox_mint_receipts','SELECT'), \
        has_table_privilege('oteryn_game_runtime','game_reward_claim_inbox_mint_lines','SELECT')")
        .fetch_one(&h.pool).await?;
    assert_eq!(private, (false, false), "the successful Cook guard grants no direct Inbox receipt access");
    Ok(())
}

pub(super) async fn entitlement(h: &Harness, character: [u8;16], skill: i16, enabled: bool) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    if enabled {
        sqlx::query("INSERT INTO game_hireling_entitlements(character_id,entitlement_id) VALUES(encode($1,'hex')::uuid,$2)")
            .bind(character.as_slice()).bind(skill).execute(&mut *tx).await?;
    } else {
        sqlx::query("DELETE FROM game_hireling_entitlements WHERE character_id=encode($1,'hex')::uuid AND entitlement_id=$2")
            .bind(character.as_slice()).bind(skill).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub(super) async fn prepare_npc(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: AdmittedSession,
    actor: ExactActorRef, hireling: [u8;16]) -> TestResult<EntityRef> {
    use crate::gameplay_transport::npc_talk::NpcTalkBook;
    use oteryn_simulation_determinism::SemanticTimeMicros;
    use std::sync::Arc;
    assert_eq!(crate::gameplay_transport::hireling::refresh(owner, actor, UseCommand {
        game_session_id: admitted.game_session_id, command_id: 300, item_fence: admitted.item_fence,
    }).await, Some(true));
    let key = crate::durability::house_hireling::npc_key(hireling);
    let npc = {
        let runtime = owner.runtime.lock().await;
        let placed = runtime.house_hireling_entities(actor, admitted.game_session_id).map_err(debug)?
            .into_iter().find(|npc| runtime.house_hireling_key(actor, admitted.game_session_id, npc.actor)
                == Some(key.as_str())).ok_or("current scoped Hireling")?;
        EntityRef { identity: placed.actor.placement_identity(), generation: placed.actor.actor_local_generation() }
    };
    owner.states.chat.install_npc_talk(Arc::new(NpcTalkBook::default()));
    owner.states.chat.admit(admitted.game_session_id, "Fixture Hero", SemanticTimeMicros::from_micros(1));
    owner.states.chat.set_npc_service(admitted.game_session_id);
    Ok(npc)
}

// Copy the source's deterministic draw only to choose an input command; the tested result still
// comes from the real private dialogue/fee/Inbox path and is asserted independently in PostgreSQL.
fn draw(owner: &ComposedFreshAdmission<'_, '_, '_>, session: crate::foundation::GameSessionId,
    command: u64, hireling: [u8;16]) -> TestResult<u8> {
    use oteryn_simulation_determinism::{GameplayDecisionRoot,DecisionOccurrenceId,deterministic_decision_u64};
    // Select only the test command's occurrence through the real native decision
    // owner and active content root. The client supplies no roll; the response
    // independently proves that production opened the intended source branch.
    let native = owner.active_generation.and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("native Cook decision root")?;
    let occurrence = Sha256::new().chain_update(b"oteryn:hireling-dish:v1\0")
        .chain_update(session.as_bytes()).chain_update(command.to_be_bytes())
        .chain_update(hireling).chain_update([6_u8]).finalize();
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence[..16].try_into()?);
    let decision = deterministic_decision_u64(&GameplayDecisionRoot::from_bytes(native.source_digest()),
        occurrence,"hireling.cook",0).map_err(debug)?;
    Ok(u8::try_from(crate::spell::uniform_draw(decision,0,5))?)
}

async fn assert_resume(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: AdmittedSession,
    npc: EntityRef, text: &str, command: u64) -> TestResult {
    let resumed = crate::gameplay_transport::npc_command::hireling_services::resume(owner, UseCommand {
        game_session_id: admitted.game_session_id, command_id: command, item_fence: admitted.item_fence,
    }, &wire::NpcTalkIntent { npc_actor: npc, text: text.into() }).await.ok_or("durable command-7 receipt")?;
    assert_eq!(resumed.disposition, wire::NpcIntentDisposition::Ok);
    assert!(resumed.inventory_changed);
    Ok(())
}

pub(super) async fn count(h: &Harness, sql: &'static str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(sql).fetch_one(&h.pool).await?)
}
async fn coins(h: &Harness) -> TestResult<i64> {
    count(h, "SELECT coalesce(sum(i.quantity * CASE i.definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.lifecycle=1").await
}

#[derive(Clone)]
pub(super) struct Talk {
    pub(super) disposition: wire::NpcIntentDisposition,
    pub(super) lines: Vec<String>,
    pub(super) window: Option<wire::NpcTradeWindow>,
}

/// One actual transport connection: the production reconnect snapshot legitimately closes
/// old conversations. Every service/menu/confirmation below stays in this same stream.
pub(super) struct Client {
    pub(super) stream: tokio::io::DuplexStream,
    pub(super) next_command: u64,
    generation: u64,
    pub(super) first_item: Option<oteryn_protocol_oteryn::item_view::ItemHandle>,
    replies: std::collections::BTreeMap<u64,Talk>,
}
impl Client {
    pub(super) fn new(stream:tokio::io::DuplexStream,admitted:AdmittedSession)->Self {
        Self {stream,next_command:admitted.continuity.next_command_id,
            generation:admitted.continuity.connection_generation,first_item:None,
            replies:std::collections::BTreeMap::new()}
    }
    pub(super) async fn close(&mut self)->TestResult {self.stream.shutdown().await?;Ok(())}
    pub(super) async fn opaque(&mut self,kind:u32,payload:&[u8],command:u64)->TestResult<(CommandStatus,Vec<u8>)> {
        let fresh=command==self.next_command;
        self.send(kind,payload,command).await?;
        loop {
            let frame=self.frame().await?;
            let envelope=decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type()!=MessageType::CommandResult {continue}
            let reply=decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(reply.command_id,command);
            if fresh {self.next_command+=1;} else {assert_eq!(reply.status,CommandStatus::DuplicateReplay);}
            return Ok((reply.status,reply.payload.to_vec()));
        }
    }

    pub(super) async fn talk(&mut self,npc:EntityRef,text:&str)->TestResult<Talk> {
        let reply=self.talk_at(npc,text,self.next_command,false).await?;
        assert_eq!(reply.disposition,wire::NpcIntentDisposition::Ok,"talk {text}: {:?}",reply.lines);
        Ok(reply)
    }
    pub(super) async fn talk_window(&mut self,npc:EntityRef,text:&str)->TestResult<Talk> {
        let reply=self.talk_at(npc,text,self.next_command,true).await?;
        assert_eq!(reply.disposition,wire::NpcIntentDisposition::Ok,"window {text}: {:?}",reply.lines);
        Ok(reply)
    }
    pub(super) async fn talk_command(&mut self,npc:EntityRef,text:&str,command:u64)->TestResult<Talk> {
        self.talk_at(npc,text,command,false).await
    }
    async fn talk_at(&mut self,npc:EntityRef,text:&str,command:u64,want_window:bool)->TestResult<Talk> {
        // Native NPC0 permits four talks in a rolling second.
        tokio::time::sleep(std::time::Duration::from_millis(260)).await;
        let fresh=command==self.next_command;
        let payload=wire::encode_npc_talk_intent(&wire::NpcTalkIntent{npc_actor:npc,text:text.into()}).map_err(debug)?;
        self.send(wire::COMMAND_TYPE_NPC_TALK_INTENT,&payload,command).await?;
        let mut accepted=false;
        let mut reply=Talk {disposition:wire::NpcIntentDisposition::Rejected,lines:Vec::new(),window:None};
        loop {
            let frame=self.frame().await?;
            let envelope=decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type()==MessageType::CommandResult {
                let result=decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id,command);
                reply.disposition=wire::decode_npc_intent_result(result.payload).map_err(debug)?;
                let expected=if !fresh {CommandStatus::DuplicateReplay}
                    else if reply.disposition==wire::NpcIntentDisposition::Rejected {CommandStatus::Rejected}
                    else {CommandStatus::Accepted};
                assert_eq!(result.status,expected,"command-7 {text}");
                if !fresh {
                    let cached=self.replies.get(&command).ok_or("original NPC reply in this connection")?.clone();
                    assert_eq!(cached.disposition,reply.disposition);return Ok(cached);
                }
                self.next_command+=1;accepted=true;
                if reply.disposition!=wire::NpcIntentDisposition::Ok {return Ok(reply);}
            } else if envelope.message_type()==MessageType::StateDelta && accepted {
                let delta=oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
                if delta.domain_id==wire::STATE_DOMAIN_NPC_CONVERSATION {
                    reply.lines=wire::decode_npc_conversation(delta.payload).map_err(debug)?.lines;
                    if !want_window {self.replies.insert(command,reply.clone());return Ok(reply);}
                } else if delta.domain_id==wire::STATE_DOMAIN_NPC_TRADE_WINDOW && want_window {
                    reply.window=Some(wire::decode_npc_trade_window(delta.payload).map_err(debug)?);
                    self.replies.insert(command,reply.clone());return Ok(reply);
                }
            }
        }
    }
    pub(super) async fn trade(&mut self,intent:&wire::NpcTradeIntent,command:u64)->TestResult<wire::NpcIntentDisposition> {
        let fresh=command==self.next_command;
        let payload=wire::encode_npc_trade_intent(intent).map_err(debug)?;
        self.send(wire::COMMAND_TYPE_NPC_TRADE_INTENT,&payload,command).await?;
        loop {
            let frame=self.frame().await?;
            let envelope=decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type()==MessageType::CommandResult {
                let result=decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id,command);
                let disposition=wire::decode_npc_intent_result(result.payload).map_err(debug)?;
                let expected=if !fresh {CommandStatus::DuplicateReplay}
                    else if disposition==wire::NpcIntentDisposition::Rejected {CommandStatus::Rejected}
                    else {CommandStatus::Accepted};
                assert_eq!(result.status,expected,"command-8");
                if fresh {self.next_command+=1;}
                return Ok(disposition);
            }
        }
    }
    async fn send(&mut self,kind:u32,payload:&[u8],command:u64)->TestResult {
        let frame=encode_client_command(self.generation,&ClientCommandValue {
            command_id:command,command_type:kind,payload,
        }).map_err(debug)?;
        self.stream.write_u32(u32::try_from(frame.len())?).await?;
        self.stream.write_all(&frame).await?;Ok(())
    }
    async fn frame(&mut self)->TestResult<Vec<u8>> {
        let length=usize::try_from(self.stream.read_u32().await?)?;assert!(length<=1 <<20);
        let mut frame=vec![0;length];self.stream.read_exact(&mut frame).await?;
        let envelope=decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type()==MessageType::SnapshotChunk {
            let (_,domains)=oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
            for domain in domains {
                if domain.domain_id==oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                    self.first_item=oteryn_protocol_oteryn::item_view::decode_character_inventory(domain.payload)
                        .map_err(debug)?.entries.first().map(|entry|entry.handle);
                }
            }
        }
        Ok(frame)
    }
}

pub(super) async fn drive<T,F>(owner:&ComposedFreshAdmission<'_, '_, '_>,admitted:AdmittedSession,
    server:&mut tokio::io::DuplexStream,client:F)->TestResult<(AdmittedSession,T)>
where F:std::future::Future<Output=TestResult<T>> {
    let mut server_run=std::pin::pin!(serve_admitted(server,admitted,owner,IDLE_LIVENESS));
    let mut client_run=std::pin::pin!(client);let mut end=None;
    let result=tokio::time::timeout(std::time::Duration::from_secs(45),std::future::poll_fn(|cx| {
        use std::future::Future;
        if end.is_none() && let std::task::Poll::Ready(value)=server_run.as_mut().poll(cx) {end=Some(value);}
        match client_run.as_mut().poll(cx) {
            std::task::Poll::Pending if end.is_some()=>std::task::Poll::Ready(Err("server ended before NPC client completed".into())),
            state=>state,
        }
    })).await?;
    if result.is_ok() && end.is_none() {end=Some(server_run.await);}
    let value=result?;
    match end.ok_or("NPC transport completion")? {
        ConnectionEnd::AdmittedThenDisconnected(current)|ConnectionEnd::AdmittedThenClosed(current,_)=>Ok((current,value)),
        other=>Err(format!("NPC connection ended unexpectedly: {other:?}").into()),
    }
}
