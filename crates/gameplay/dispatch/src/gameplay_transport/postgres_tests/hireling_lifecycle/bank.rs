//! Existing Bank command7 talks through a scoped Hireling and the native Bank owner.
use super::{Harness, TestResult, debug, id};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession, UseCommand};
use crate::foundation::ExactActorRef;
use super::cook::{Client, drive};

// Same independent-account recipient as the original native Bank fixture.
const FRIEND: u8 = 61;
const FRIEND_ACCOUNT: u8 = 60;

pub(super) async fn scenario(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession, actor: ExactActorRef, hireling: [u8; 16]) -> TestResult {
    use crate::gameplay_transport::npc_talk::NpcTalkBook;
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
    use oteryn_simulation_determinism::SemanticTimeMicros;
    use std::sync::Arc;
    super::super::item_look::seed_main_backpack(h, &[("oteryn:item.tibia.i3031", 20)]).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_hireling_entitlements(character_id,entitlement_id) VALUES(encode($1,'hex')::uuid,1001)")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    // bank_postgres_cases.rs::seed_bank supplies a live Bank Friend on another
    // Account in this World. Copy current root fields without its generated
    // name_key; no recipient session or synthetic command is required.
    sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES(encode($1,'hex')::uuid)")
        .bind(id(FRIEND_ACCOUNT).as_slice()).execute(&mut *tx).await?;
    let friend: String = sqlx::query_scalar("SELECT (to_jsonb(r)||jsonb_build_object(\
        'character_id',encode($2,'hex')::uuid,'account_id',encode($3,'hex')::uuid,'name','Bank Friend'))::text \
        FROM game_character_roots r WHERE r.character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(id(FRIEND).as_slice()).bind(id(FRIEND_ACCOUNT).as_slice())
        .fetch_one(&mut *tx).await?;
    super::insert_character_root_copy(&mut tx, &friend).await?;
    tx.commit().await?;
    let refreshed = crate::gameplay_transport::hireling::refresh(owner, actor, UseCommand {
        game_session_id: admitted.game_session_id, command_id: 300, item_fence: admitted.item_fence,
    }).await;
    assert_eq!(refreshed, Some(true));
    let key = crate::durability::house_hireling::npc_key(hireling);
    let npc = {
        let runtime = owner.runtime.lock().await;
        let placed = runtime.house_hireling_entities(actor, admitted.game_session_id).map_err(debug)?
            .into_iter().find(|npc| runtime.house_hireling_key(actor, admitted.game_session_id, npc.actor)
                == Some(key.as_str())).ok_or("scoped Hireling NPC")?;
        EntityRef { identity: placed.actor.placement_identity(), generation: placed.actor.actor_local_generation() }
    };
    owner.states.chat.install_npc_talk(Arc::new(NpcTalkBook::default()));
    owner.states.chat.admit(admitted.game_session_id, "Fixture Hero", SemanticTimeMicros::from_micros(1));
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let (mut server, stream) = tokio::io::duplex(1 << 20);
    let mut client = Client::new(stream, admitted);
    let client_run = async {
        for text in ["hi", "bank", "balance", "deposit 10 gold", "yes"] {
            let reply = client.talk(npc, text).await?;
            if text == "balance" {
                assert!(reply.lines.iter().any(|line| line == "Your account balance is 0 gold."),
                    "bank balance reply: {:?}", reply.lines);
            }
            if text == "deposit 10 gold" {
                assert!(reply.lines.iter().any(|line| line == "Do you want to deposit 10 gold?"),
                    "the exact native Bank grammar opens a deposit confirmation: {:?}", reply.lines);
            }
        }
        let deposit_command = client.next_command - 1;
        client.talk_command(npc, "yes", deposit_command).await?;
        assert_eq!(balance(h).await?, 10, "duplicate confirmation deposits once");
        let deposited = client.talk(npc, "balance").await?;
        assert!(deposited.lines.iter().any(|line| line == "Your account balance is 10 gold."),
            "balance reflects the native deposit: {:?}", deposited.lines);
        for text in ["withdraw 5 gold", "yes"] {
            let reply = client.talk(npc, text).await?;
            if text == "withdraw 5 gold" {
                assert!(reply.lines.iter().any(|line| line == "Do you want to withdraw 5 gold?"),
                    "the exact native Bank grammar opens a withdrawal confirmation: {:?}", reply.lines);
            }
        }
        let withdraw_command = client.next_command - 1;
        client.talk_command(npc, "yes", withdraw_command).await?;
        assert_eq!(balance(h).await?, 5, "duplicate confirmation withdraws once");
        for text in ["transfer 3 gold to Bank Friend", "yes"] { client.talk(npc, text).await?; }
        let transfer_command = client.next_command - 1;
        client.talk_command(npc, "yes", transfer_command).await?;
        assert_eq!(balance(h).await?, 2, "duplicate transfer debits the sender once");
        let received: i64 = sqlx::query_scalar("SELECT balance FROM game_account_bank_balances \
            WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
            .bind(id(FRIEND_ACCOUNT).as_slice()).bind(admitted.world_id.as_bytes().as_slice())
            .fetch_one(&h.pool).await?;
        assert_eq!(received, 3, "the actual recipient account is credited once");
        // Bypass the connection cache as well: the existing accepted-command
        // owner must replay its frozen transfer instead of creating new value.
        let original = crate::foundation::CommandRef::new(admitted.game_session_id,
            crate::foundation::CommandId::new(transfer_command).map_err(debug)?);
        let completion = owner.root.resume_npc_bank(owner.character, owner.holder,
            admitted.item_fence.ok_or("House Bank fence")?, original).await.map_err(debug)?
            .ok_or("durable transfer confirmation")?;
        let crate::durability::bank::BankOperationOutcome::Replayed(record) = completion.outcome else {
            return Err("native House transfer must replay its original receipt".into());
        };
        assert_eq!(record.entries.len(), 2);
        assert!(record.coin_lines.is_empty(), "a transfer never touches physical coins");
        let pair: (bool, bool) = sqlx::query_as("SELECT bool_and(o.counterpart_entry_id=i.entry_id),\
            bool_and(i.counterpart_entry_id=o.entry_id) FROM game_account_bank_entries o \
            JOIN game_account_bank_entries i ON i.transaction_id=o.transaction_id AND i.kind=4 \
            WHERE o.kind=3")
            .fetch_one(&h.pool).await?;
        assert_eq!(pair, (true, true), "both transfer entries commit as one native pair");
        let operations: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_bank_operations WHERE acting_character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(operations, 3);
        let physical: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity * CASE i.definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.lifecycle=1")
            .fetch_one(&h.pool).await?;
        assert_eq!(physical, 15, "deposit burns ten physical coins and withdrawal returns five");
        let envelopes: Vec<Vec<u8>> = sqlx::query_scalar("SELECT envelope FROM game_account_bank_audit_outbox")
            .fetch_all(&h.pool).await?;
        assert_eq!(envelopes.len(), 3, "replays append no second Bank audit");
        let mut transfers = 0;
        for bytes in envelopes {
            let (_, value) = crate::durability::bank_audit::decode_bank_envelope(&bytes).map_err(debug)?;
            let house = value.hireling_house.ok_or("actual House Bank scope in audit")?;
            assert_eq!(house.house_key, super::HOUSE);
            assert_eq!(house.hireling_id.as_slice(), hireling.as_slice());
            assert_eq!(value.channel_id.as_slice(), admitted.channel_id.as_bytes().as_slice());
            assert_eq!(value.runtime_scope_ownership_generation,
                admitted.item_fence.ok_or("current House fence")?.scope_ownership_generation.get());
            if !value.recipient_character_id.is_empty() {
                transfers += 1;
                assert_eq!(value.recipient_character_id.as_slice(), id(FRIEND).as_slice());
                assert_eq!(value.amount, 3);
                assert!(value.backpack_item_instance_id.is_empty());
                assert!(value.coin_lines.is_empty());
                let entries: Vec<_> = value.value_lines.iter().map(|line|
                    (line.kind, line.amount, line.balance_before, line.balance_after)).collect();
                assert_eq!(entries, vec![(3, 3, 5, 2), (4, 3, 0, 3)],
                    "named-House transfer audit preserves native sender/recipient value lines");
            }
        }
        assert_eq!(transfers, 1);
        // Separate native-owner continuation seam: accept a resolved request
        // while this genuine source controller/NPC context is still current,
        // then close before the money writer is invoked. No SQL session/history
        // fixture manufactures the later recovery or updates this confirmation.
        let pending = enqueue_pending(h, owner, admitted, actor, npc, client.next_command).await?;
        client.close().await?;
        Ok(pending)
    };
    let (admitted, pending) = drive(owner, admitted, &mut server, client_run).await?;
    continue_after_resume(h, owner, admitted, pending).await?;
    Ok(())
}

async fn balance(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=(SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid) AND world_id=encode($2,'hex')::uuid")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?)
}



struct NativePending {
    command: crate::foundation::CommandRef,
    transaction: [u8;16],
    confirmation: String,
}

async fn enqueue_pending(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession, actor: ExactActorRef,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    command_id: u64) -> TestResult<NativePending> {
    use crate::durability::bank::{BankCoinFacts,BankNpcSource,BankRequest};
    use crate::foundation::{CommandId,CommandRef};
    let fence = admitted.item_fence.ok_or("pending Bank current House fence")?;
    // Match the actual confirming command's serialization boundary. The live
    // connection must not race this native seam with a background gameplay
    // write, and its current NPC/Content facts stay held through acceptance.
    let permit = owner.spell_lane_permit().await.ok_or("native Bank enqueue lane")?;
    let runtime = owner.runtime.lock().await;
    let intent = oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
        npc_actor: npc, text: "balance".into(),
    };
    let context = owner.states.npc_conversations.current_bank(&owner.states.chat, &runtime, actor,
        admitted.game_session_id, &intent).ok_or("current native Bank conversation")?;
    let (gold,platinum,crystal) =
        crate::gameplay_transport::npc_command::bank_coin_definitions(owner,&runtime)
            .ok_or("actual pinned coin definitions")?;
    let backpack = owner.root.read_character_backpack(owner.character,fence.character_id)
        .await.map_err(debug)?.ok_or("current backpack")?;
    let native = owner.active_generation.and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("actual native content")?;
    let backpack = crate::gameplay_transport::item_ref_admission::generation_item_facts(
        native,&backpack.backpack.definition).map_err(debug)?;
    let request = crate::gameplay_transport::npc_command::bank_request(
        &crate::gameplay_transport::bank_npc::BankNpcOperation::Deposit(2),
        Some(BankCoinFacts {gold,platinum,crystal,backpack}),admitted.game_session_id,command_id)
        .ok_or("native resolved Bank request")?;
    let BankRequest::Deposit(deposit) = &request else {return Err("deposit shape".into());};
    let transaction = deposit.identity.transaction_id;
    let command = CommandRef::new(admitted.game_session_id,CommandId::new(command_id).map_err(debug)?);
    owner.root.enqueue_npc_bank(owner.character,owner.holder,fence,BankNpcSource {
        command,npc_identity:npc.identity,npc_key:context.2,npc_generation:npc.generation,
        catalogue_revision:context.0,prompt_generation:context.1,recipient_name:None,
    },request).await.map_err(debug)?;
    drop(runtime);
    drop(permit);
    assert_eq!(balance(h).await?,2,"enqueue alone cannot mutate money");
    let row: (String,bool) = sqlx::query_as("SELECT (to_jsonb(c)-'completed')::text,completed \
        FROM game_npc_bank_confirmations c WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(admitted.game_session_id.as_bytes().as_slice()).bind(command_id.to_string())
        .fetch_one(&h.pool).await?;
    assert!(!row.1);
    Ok(NativePending {command,transaction,confirmation:row.0})
}

async fn continue_after_resume(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession, pending: NativePending) -> TestResult {
    use crate::durability::bank::{BankError,BankOperationOutcome};
    use crate::gameplay_transport::connection::ControlLossResult;
    let old = admitted.item_fence.ok_or("original Bank fence")?;
    assert_eq!(owner.lose_control_for_cause(admitted,std::time::Duration::ZERO,None).await,
        ControlLossResult::Recorded);
    let resumed = super::position::resume(owner,admitted,admitted.game_session_id,7,10).await?;
    let current = resumed.item_fence.ok_or("actual resumed Bank fence")?;
    assert_eq!(current.game_session_id,old.game_session_id);
    assert_eq!(current.connection_generation.get(),old.connection_generation.get()+1);
    assert_eq!(resumed.continuity.next_command_id,admitted.continuity.next_command_id);
    let mut old_connection_only = current;
    old_connection_only.connection_generation = old.connection_generation;
    assert_eq!(old_connection_only,old,
        "the first negative differs from current authority only in connection generation");
    assert!(matches!(owner.root.resume_npc_bank(owner.character,owner.holder,old,pending.command).await,
        Err(BankError::AuthorityRejected)),"the original connection is no longer authority");
    let mut stale_lease = current;
    stale_lease.character_lease_generation = current.character_lease_generation
        .checked_sub(1).filter(|generation|*generation>0).ok_or("prior positive House lease")?;
    assert!(matches!(owner.root.resume_npc_bank(owner.character,owner.holder,stale_lease,pending.command).await,
        Err(BankError::AuthorityRejected)));
    assert_eq!(balance(h).await?,2);
    let missing: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_bank_operations WHERE transaction_id=encode($1,'hex')::uuid")
        .bind(pending.transaction.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(missing,0,"stale pending retries cannot produce a money receipt");
    let completion = owner.root.resume_npc_bank(owner.character,owner.holder,current,pending.command)
        .await.map_err(debug)?.ok_or("accepted native continuation")?;
    assert!(matches!(completion.outcome,BankOperationOutcome::Committed(_)));
    assert_eq!(balance(h).await?,4);
    let proof: (String,String) = sqlx::query_as("SELECT c.origin_connection_generation::text,o.source_connection_generation::text \
        FROM game_npc_bank_confirmations c JOIN game_account_bank_operations o USING(operation_occurrence_id) \
        WHERE o.transaction_id=encode($1,'hex')::uuid")
        .bind(pending.transaction.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(proof,(old.connection_generation.get().to_string(),current.connection_generation.get().to_string()),
        "immutable origin and independently current deciding generation are both retained");
    let row: (String,bool) = sqlx::query_as("SELECT (to_jsonb(c)-'completed')::text,completed \
        FROM game_npc_bank_confirmations c WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(admitted.game_session_id.as_bytes().as_slice()).bind(pending.command.command_id().get().to_string())
        .fetch_one(&h.pool).await?;
    assert_eq!(row,(pending.confirmation,true),"only terminal completion may change the accepted confirmation");
    let replay = owner.root.resume_npc_bank(owner.character,owner.holder,current,pending.command)
        .await.map_err(debug)?.ok_or("retained native outcome")?;
    assert!(matches!(replay.outcome,BankOperationOutcome::Replayed(_)));
    assert_eq!(balance(h).await?,4);
    let events: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_bank_audit_outbox WHERE transaction_id=encode($1,'hex')::uuid")
        .bind(pending.transaction.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(events,1);
    Ok(())
}
