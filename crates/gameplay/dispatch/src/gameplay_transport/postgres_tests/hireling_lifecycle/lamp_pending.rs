//! Actual native commits at the owner publication boundary, followed by literal USE replay.
use super::{Harness,TestResult,debug};
use crate::durability::house_hireling::{HirelingLampInput,HirelingLampPlacement,HirelingReceiptRead,HIRELING_LAMP_KEY,command_identity};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_use::{ItemUseFlaskPlan,ItemUseKind,ItemUseRequest};
use crate::foundation::{CommandId,CommandRef,ExactActorRef,MessageType,decode_wire_envelope};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession,ConnectionEnd,FreshAdmissionAuthority,IDLE_LIVENESS,UseCommand,serve_admitted};
use crate::gameplay_transport::hireling::PendingLampUse;
use oteryn_protocol_oteryn::{CommandStatus,hireling::management::{self,Reply,Refusal,Request}};
use oteryn_protocol_oteryn::world_object::{self as wire,UseDisposition,UseIntent,UseTarget};
use std::num::NonZeroU32;
use tokio::io::{AsyncReadExt,AsyncWriteExt};

pub(super) async fn scenario(h:&Harness,owner:&ComposedFreshAdmission<'_,'_,'_>,mut admitted:AdmittedSession,
    actor:ExactActorRef,hireling:[u8;16])->TestResult {
    let definition={let runtime=owner.runtime.lock().await;
        let index=crate::gameplay_transport::item_ref_admission::pinned_index(&runtime,owner.active_generation).ok_or("native index")?;
        index.revision(HIRELING_LAMP_KEY).and_then(|revision|index.definition_ref("Item",HIRELING_LAMP_KEY,revision))
            .ok_or("lamp definition index")?};
    let body=wire::encode_item_use_intent(&UseIntent {target:UseTarget::ItemByDefinition(definition),
        use_with:None,use_with_map_item:None,use_with_position:None}).map_err(debug)?;
    let plan=frozen_plan(owner,actor,admitted,hireling,definition).await?;
    let command=use_command(admitted);
    assert_eq!(owner.root.read_hireling_receipt_state(owner.character,command.item_fence.ok_or("fence")?,
        command.command_id,plan.input.binding()).await.map_err(debug)?,HirelingReceiptRead::Missing);
    park(owner,actor,command,plan.clone()).await?;
    // Model cancellation after the real native COMMIT and before runtime publication.
    assert_eq!(owner.root.commit_hireling_lamp(owner.character,owner.holder,
        command.item_fence.ok_or("fence")?,plan.clone()).await.map_err(debug)?,Reply::Committed);
    assert_eq!(super::uses(h).await?,1);
    assert_eq!(super::inbox_count(h).await?,0);
    assert_eq!(owner.root.read_hireling_receipt_state(owner.character,command.item_fence.ok_or("fence")?,
        command.command_id,plan.input.binding()).await.map_err(debug)?,HirelingReceiptRead::Found(Reply::Committed));
    let mut changed_binding=plan.input.binding();changed_binding[0]^=1;
    assert_eq!(owner.root.read_hireling_receipt_state(owner.character,command.item_fence.ok_or("fence")?,
        command.command_id,changed_binding).await.map_err(debug)?,HirelingReceiptRead::Conflict);
    assert!(owner.root.read_hireling_bound_receipt(owner.character,command.item_fence.ok_or("fence")?,
        command.command_id,changed_binding).await.is_err(),"the old receipt API retains its conflict contract");
    assert!(owner.item_use_pending(actor,admitted.game_session_id).await);
    assert!(owner.runtime.lock().await.house_hireling_entities(actor,admitted.game_session_id).map_err(debug)?.is_empty(),
        "the durable value is committed while native projection has not been published");
    let original_audit=audit(h,plan.item_use.transaction_id).await?;
    // Real PostgreSQL contention makes the same receipt read unavailable. Ingress must
    // close without terminalizing USE, retaining its exact CommandRef for the retry.
    let mut blocker=h.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:hireling:'||encode($1,'hex')||':'||$2,0))")
        .bind(admitted.game_session_id.as_bytes().as_slice()).bind(command.command_id.to_string())
        .execute(&mut *blocker).await?;
    unacknowledged_wire(owner,&mut admitted,&body).await.map_err(|error|format!("unknown native receipt must close without result: {error}"))?;
    assert_eq!(admitted.continuity.next_command_id,command.command_id);
    let original=CommandRef::new(command.game_session_id,CommandId::new(command.command_id).map_err(debug)?);
    assert!(owner.states.command_ingress.pending_reconciliation(original),
        "unknown retains the exact inactive outstanding reservation");
    assert!(!owner.states.command_ingress.resumable(&admitted));
    assert!(owner.item_use_pending(actor,admitted.game_session_id).await);
    blocker.rollback().await?;
    // The deliberately expired semantic pass may retire its only DB holder.
    // This direct owner fixture has no node loop; run the same one demanded
    // maintenance window as the node, without opening connections in ingress.
    assert!(owner.root.maintain_ready_once().await.map_err(debug)?,
        "the native root restores its demanded holder after the injected timeout");
    assert!(owner.root.is_ready(), "receipt recovery starts with a ready native holder");
    assert_eq!(use_wire(owner,&mut admitted,&body).await.map_err(|error|format!("recover disconnected original committed USE: {error}"))?,(CommandStatus::DuplicateReplay,UseDisposition::Committed));
    assert_eq!(admitted.continuity.next_command_id,command.command_id+1);
    assert!(owner.states.command_ingress.resumable(&admitted),
        "only proven native completion restores the original stream's continuity");
    let mut unrelated_counter=admitted;
    unrelated_counter.continuity.next_command_id=command.command_id.checked_sub(1).ok_or("prior native STEP")?;
    let unchanged=unrelated_counter.continuity;
    assert!(owner.states.command_ingress.recover_trade_continuity(&mut unrelated_counter).is_none(),
        "a retained completion cannot reconstruct an unrelated caller counter");
    assert_eq!(unrelated_counter.continuity,unchanged);
    assert!(!owner.item_use_pending(actor,admitted.game_session_id).await);
    assert_eq!(owner.runtime.lock().await.house_hireling_entities(actor,admitted.game_session_id).map_err(debug)?.len(),1);
    assert_eq!(super::uses(h).await?,1);
    assert_eq!(audit(h,plan.item_use.transaction_id).await?,original_audit);
    assert_eq!(use_wire_at(owner,&mut admitted,&body,command.command_id).await.map_err(|error|format!("replay committed USE through current stream: {error}"))?,
        (CommandStatus::DuplicateReplay,UseDisposition::Committed));
    // Also bypass the wire replay cache entirely.
    assert_eq!(owner.use_item_definition(actor,command,definition).await.disposition,UseDisposition::Committed);
    assert_eq!(super::uses(h).await?,1);

    // A known native refusal releases its parked work and remains a refusal on replay.
    let refused_command=use_command(admitted);
    let mut refused=plan.clone();
    refused.item_use.command_id=refused_command.command_id;
    let reference=CommandRef::new(refused_command.game_session_id,CommandId::new(refused_command.command_id).map_err(debug)?);
    refused.item_use.transaction_id=command_identity(reference,b"lamp-place");
    refused.item_use.event_id=command_identity(reference,b"lamp-place-event");
    park(owner,actor,refused_command,refused.clone()).await?;
    assert_eq!(owner.root.commit_hireling_lamp(owner.character,owner.holder,
        refused_command.item_fence.ok_or("fence")?,refused).await.map_err(debug)?,Reply::Refused(Refusal::Conflict));
    assert_eq!(use_wire(owner,&mut admitted,&body).await.map_err(|error|format!("publish known native refusal: {error}"))?,(CommandStatus::Rejected,UseDisposition::Rejected));
    assert!(!owner.item_use_pending(actor,admitted.game_session_id).await);
    assert_eq!(use_wire_at(owner,&mut admitted,&body,refused_command.command_id).await.map_err(|error|format!("replay known native refusal: {error}"))?,
        (CommandStatus::DuplicateReplay,UseDisposition::Rejected));
    assert_eq!(super::uses(h).await?,1);

    let returned=super::wire::exchange(owner,&mut admitted,management::COMMAND_TYPE_HIRELING_MANAGEMENT,
        &Request::ReturnToLamp {hireling}.encode()).await.map_err(|error|format!("return active hireling to lamp before pending pre-commit retry: {error}"))?;
    assert_eq!(returned.0,CommandStatus::Accepted);
    assert_eq!(Reply::decode(&returned.1),Some(Reply::Committed));
    assert_eq!(super::inbox_count(h).await?,1);

    // Cancellation before the native COMMIT retains every original audit/input field.
    // A real USE resolves that exact plan rather than selecting a new source or clock.
    let retry=frozen_plan(owner,actor,admitted,hireling,definition).await?;
    park(owner,actor,use_command(admitted),retry.clone()).await?;
    assert_eq!(use_wire(owner,&mut admitted,&body).await.map_err(|error|format!("execute exact frozen pre-commit USE: {error}"))?,(CommandStatus::Accepted,UseDisposition::Committed));
    assert!(!owner.item_use_pending(actor,admitted.game_session_id).await);
    assert_eq!(super::uses(h).await?,2);
    assert_eq!(super::inbox_count(h).await?,0);
    let (envelope,event)=crate::durability::item_use_audit::decode_item_use_envelope(&audit(h,retry.item_use.transaction_id).await?).map_err(debug)?;
    assert_eq!(envelope.event_id,retry.item_use.event_id.to_vec());
    assert_eq!(envelope.occurred_at_unix_ms,retry.item_use.occurred_at_unix_ms);
    assert_eq!(event.source_before.as_ref().ok_or("source")?.item_instance_id,retry.item_use.source_item_instance_id.to_vec());
    assert_eq!(event.house_key.as_deref(),Some(super::HOUSE));
    Ok(())
}

fn use_command(admitted:AdmittedSession)->UseCommand {
    UseCommand {game_session_id:admitted.game_session_id,command_id:admitted.continuity.next_command_id,item_fence:admitted.item_fence}
}

async fn frozen_plan(owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,admitted:AdmittedSession,
    hireling:[u8;16],definition:NonZeroU32)->TestResult<HirelingLampPlacement> {
    let current=admitted.item_fence.ok_or("fence")?;
    let saved=owner.root.read_owned_hirelings(owner.character,owner.holder,current,*owner.channel_id.as_bytes()).await.map_err(debug)?
        .into_iter().find(|row|row.id==hireling).ok_or("owned identity")?;
    let lamp=saved.lamp.ok_or("physical lamp")?;
    let state=owner.runtime.lock().await.house_actor_state(actor,admitted.game_session_id).map_err(debug)?;
    let inventory=owner.root.read_hireling_use_inventory(owner.character,owner.holder,current,*owner.channel_id.as_bytes()).await.map_err(debug)?;
    let source_definition={let runtime=owner.runtime.lock().await;
        let index=crate::gameplay_transport::item_ref_admission::pinned_index(&runtime,owner.active_generation).ok_or("native index")?;
        let (key,revision)=index.definition_at(definition).ok_or("source definition")?;
        TypedDefinitionRef {family:"Item".into(),production_key:key.into(),revision_ref:revision.into()}};
    let reference=CommandRef::new(admitted.game_session_id,CommandId::new(admitted.continuity.next_command_id).map_err(debug)?);
    Ok(HirelingLampPlacement {hireling,input:HirelingLampInput::ItemDefinition(definition.get()),house:state.house,
        origin_channel:*owner.channel_id.as_bytes(),position:(u16::try_from(state.position.x)?,u16::try_from(state.position.y)?,-i8::try_from(state.position.floor)?),
        item_use:ItemUseRequest {kind:ItemUseKind::HirelingLamp {origin_channel:*owner.channel_id.as_bytes()},
            command_id:admitted.continuity.next_command_id,content_revision:inventory.content_revision,ruleset_revision:inventory.ruleset_revision,
            sim_revision:crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),transaction_id:command_identity(reference,b"lamp-place"),
            event_id:command_identity(reference,b"lamp-place-event"),occurred_at_unix_ms:1_700_000_000_001,
            server_build_id:crate::durability::character_authority::SERVER_BUILD_ID.into(),source_item_instance_id:lamp,
            source_definition,source_quantity_before:1,flask:ItemUseFlaskPlan::None}})
}

async fn park(owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,command:UseCommand,
    plan:HirelingLampPlacement)->TestResult {
    let _runtime=owner.runtime.lock().await;
    let mut states=owner.states.spell_states.lock().await;
    assert!(PendingLampUse::prepared(actor,command,plan).ok_or("pending plan")?.park(&mut states));
    assert!(states.has_pending_spell_commit(actor,command.game_session_id));
    Ok(())
}

async fn audit(h:&Harness,transaction:[u8;16])->TestResult<Vec<u8>> {
    Ok(sqlx::query_scalar("SELECT envelope FROM game_item_audit_outbox WHERE transaction_id=encode($1,'hex')::uuid")
        .bind(transaction.as_slice()).fetch_one(&h.pool).await?)
}

async fn use_wire(owner:&ComposedFreshAdmission<'_,'_,'_>,admitted:&mut AdmittedSession,
    body:&[u8])->TestResult<(CommandStatus,UseDisposition)> {
    let (status,payload)=super::wire::exchange(owner,admitted,wire::COMMAND_TYPE_USE_INTENT,body).await?;
    Ok((status,wire::decode_use_result(&payload).map_err(debug)?))
}

async fn use_wire_at(owner:&ComposedFreshAdmission<'_,'_,'_>,admitted:&mut AdmittedSession,
    body:&[u8],command:u64)->TestResult<(CommandStatus,UseDisposition)> {
    // Re-send the actual old command through the current admitted stream. Never
    // rewind its high-water/sequence counters to impersonate a fresh namespace.
    let input=*admitted;
    let (mut server,stream)=tokio::io::duplex(1<<20);
    let mut client=super::cook::Client::new(stream,input);
    let request=async {
        let (status,payload)=client.opaque(wire::COMMAND_TYPE_USE_INTENT,body,command).await?;
        client.close().await?;
        Ok((status,wire::decode_use_result(&payload).map_err(debug)?))
    };
    let (current,result)=super::cook::drive(owner,input,&mut server,request).await?;
    *admitted=current;
    Ok(result)
}

async fn unacknowledged_wire(owner:&ComposedFreshAdmission<'_,'_,'_>,admitted:&mut AdmittedSession,
    body:&[u8])->TestResult {
    let frame=oteryn_protocol_oteryn::encode_client_command(admitted.continuity.connection_generation,
        &oteryn_protocol_oteryn::ClientCommandValue {command_id:admitted.continuity.next_command_id,
            command_type:wire::COMMAND_TYPE_USE_INTENT,payload:body}).map_err(debug)?;
    let (mut server,mut client)=tokio::io::duplex(1<<20);
    let input=*admitted;
    let server=async {let end=serve_admitted(&mut server,input,owner,IDLE_LIVENESS).await;drop(server);end};
    let client=async {
        client.write_all(&u32::try_from(frame.len())?.to_be_bytes()).await?;
        client.write_all(&frame).await?;
        loop {
            let length=match client.read_u32().await {
                Ok(length)=>usize::try_from(length)?,
                Err(error) if error.kind()==std::io::ErrorKind::UnexpectedEof=>break,
                Err(error)=>return Err(error.into()),
            };
            let mut bytes=vec![0;length];client.read_exact(&mut bytes).await?;
            let envelope=decode_wire_envelope(&bytes).map_err(debug)?;
            assert_ne!(envelope.message_type(),MessageType::CommandResult,
                "an unresolved native commit must not be cached as a terminal USE refusal");
        }
        Ok::<_,Box<dyn std::error::Error>>(())
    };
    let mut server=std::pin::pin!(server);
    let mut client=std::pin::pin!(client);
    let mut server_end=None;
    let mut client_result=None;
    let (end,result)=tokio::time::timeout(std::time::Duration::from_secs(20),std::future::poll_fn(|cx| {
        use std::future::Future;
        if server_end.is_none() && let std::task::Poll::Ready(end)=server.as_mut().poll(cx) {
            server_end=Some(end);
        }
        if client_result.is_none() && let std::task::Poll::Ready(result)=client.as_mut().poll(cx) {
            client_result=Some(result);
        }
        if server_end.is_some() && client_result.is_some() {
            return std::task::Poll::Ready((server_end.take(),client_result.take()));
        }
        std::task::Poll::Pending
    })).await?;
    let end=end.ok_or("pending USE server completion")?;
    let result=result.ok_or("pending USE client completion")?;
    result?;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(current)|ConnectionEnd::AdmittedThenClosed(current,_)=>*admitted=current,
        other=>return Err(format!("pending USE connection end: {other:?}").into()),
    }
    Ok(())
}
