//! Literal frames perform every new action. Lost ingress refuses resumed input;
//! the native owner separately retains the immutable receipts of those actions.
use super::{Harness, ReconciledCharacterAuthority, TestResult, TypedDefinitionRef,
    RewardClaimInboxMintOutput, HIRELING_LAMP_KEY, debug, id};
use crate::foundation::{ExactActorRef, MessageType, decode_wire_envelope};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession, ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, UseCommand, serve_admitted};
use crate::durability::house_hireling::{HirelingLampInput, HirelingReceiptRead};
use oteryn_protocol_oteryn::{ClientCommandValue, CommandStatus, decode_command_result, encode_client_command};
use oteryn_protocol_oteryn::hireling::management::{self as management, Reply, Request};
use oteryn_protocol_oteryn::world_object::{self as use_wire, UseDisposition, UseIntent, UseTarget};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) struct Replay {
    initial: AdmittedSession,
    use_command: UseCommand,
    use_input: HirelingLampInput,
    management: Vec<RecordedManagement>,
    hireling: [u8; 16],
}

struct RecordedManagement {
    admitted: AdmittedSession,
    request: Request,
    answer: Reply,
}

pub(super) async fn scenario(h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>,
    owner: &ComposedFreshAdmission<'_, '_, '_>, mut admitted: AdmittedSession,
    actor: ExactActorRef) -> TestResult<Replay> {
    let current = admitted.item_fence.ok_or("House fence")?;
    let hireling = id(141);
    let lamp = id(142);
    let (definition, definition_index) = {
        let runtime = owner.runtime.lock().await;
        let index = crate::gameplay_transport::item_ref_admission::pinned_index(&runtime,
            owner.active_generation).ok_or("native item index")?;
        let revision = index.revision(HIRELING_LAMP_KEY).ok_or("canonical lamp definition")?;
        (TypedDefinitionRef { family: "Item".into(), production_key: HIRELING_LAMP_KEY.into(),
            revision_ref: revision.into() }, index.definition_ref("Item", HIRELING_LAMP_KEY, revision)
                .ok_or("wire lamp definition index")?)
    };
    let native = owner.active_generation.and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("native gameplay")?;
    let carried = crate::gameplay_transport::item_ref_admission::generation_item_facts(native, &definition)
        .map_err(debug)?;
    assert_eq!(carried.stack, super::ItemStackClass::Unknown,
        "the source-immovable lamp must not gain generic carried inventory admission");
    let facts = crate::gameplay_transport::item_ref_admission::generation_inbox_item_facts(native, &definition)
        .map_err(debug)?;
    assert_eq!(facts.stack, super::ItemStackClass::NonStackable);
    assert_eq!(facts.weight, Some(1000));
    let source = super::inbox_source(current, admitted.channel_id, 101, 143, vec![lamp])?;
    let mut mint = super::mint_request(vec![RewardClaimInboxMintOutput {
        source_line: 0, item: facts, quantity: 1, key_number: None,
    }]);
    mint.claim = definition;
    super::native_mint(h, authority, current, &source, mint).await?;
    super::seed_identity(h, hireling, lamp).await?;
    let use_bytes = use_wire::encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(definition_index), use_with: None,
        use_with_map_item: None, use_with_position: None,
    }).map_err(debug)?;
    // The real House handoff lands on its door. Donor placement forbids that
    // cell; refusal must keep the genuine Store Inbox lamp and inactive record.
    let refused = exchange(owner, &mut admitted, use_wire::COMMAND_TYPE_USE_INTENT, &use_bytes).await?;
    assert_eq!(refused.0, CommandStatus::Rejected);
    assert_eq!(use_wire::decode_use_result(&refused.1).map_err(debug)?, UseDisposition::Rejected);
    assert!(!super::active(h, hireling).await?);
    assert_eq!(super::inbox_count(h).await?, 1);
    assert_eq!(super::uses(h).await?, 0);
    assert!(owner.runtime.lock().await.house_hireling_entities(actor,
        admitted.game_session_id).map_err(debug)?.is_empty());
    enter_interior(owner, &mut admitted, actor).await?;
    // Replaying the durable operations starts after the independent native STEP.
    let initial = admitted;
    let mut management = Vec::new();
    let Reply::State(entries) = record_manage(owner, &mut admitted, &mut management, Request::List).await? else {
        return Err("management List state".into());
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].hireling, hireling);
    assert_eq!(entries[0].position, None);
    let use_command = UseCommand { game_session_id: admitted.game_session_id,
        command_id: admitted.continuity.next_command_id, item_fence: admitted.item_fence };
    let use_input = HirelingLampInput::ItemDefinition(definition_index.get());
    let used = exchange(owner, &mut admitted, use_wire::COMMAND_TYPE_USE_INTENT, &use_bytes).await?;
    assert_eq!(used.0, CommandStatus::Accepted);
    assert_eq!(use_wire::decode_use_result(&used.1).map_err(debug)?, UseDisposition::Committed);
    assert!(super::active(h, hireling).await?);
    assert_eq!(super::inbox_count(h).await?, 0);
    let Reply::Projection(bindings) = record_manage(owner, &mut admitted, &mut management, Request::Projection).await? else {
        return Err("management Projection".into());
    };
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].hireling, hireling);
    assert_ne!(bindings[0].entity.generation, 0);
    let outfit = Request::Outfit { hireling, look_type: 1108, colors: [8, 7, 6, 5] };
    assert_eq!(record_manage(owner, &mut admitted, &mut management, outfit).await?, Reply::Committed);
    let Reply::State(entries) = record_manage(owner, &mut admitted, &mut management, Request::List).await? else {
        return Err("placed management List".into());
    };
    let house = owner.runtime.lock().await.house_actor_state(actor, admitted.game_session_id).map_err(debug)?;
    assert_eq!(entries[0].position, Some((u16::try_from(house.position.x)?,
        u16::try_from(house.position.y)?, -i8::try_from(house.position.floor)?)));
    assert_eq!(entries[0].colors, [8, 7, 6, 5]);
    let returned = Request::ReturnToLamp { hireling };
    assert_eq!(record_manage(owner, &mut admitted, &mut management, returned).await?, Reply::Committed);
    assert!(!super::active(h, hireling).await?);
    assert_eq!(super::inbox_count(h).await?, 1);
    // A later real STEP changes the actor's tile too. Replaying the old USE
    // must bind only its ingress bytes, not this new tile or the returned lamp.
    let step = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
        oteryn_protocol_oteryn::world_spatial::StepDirection::East);
    let (status, _) = exchange(owner, &mut admitted,
        oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, &step).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let moved = owner.runtime.lock().await.house_actor_state(actor,
        admitted.game_session_id).map_err(debug)?;
    assert_eq!((moved.position.x, moved.position.y, moved.position.floor), (32311, 32804, 7));
    // Retry only this already-committed native operation under its genuine
    // actor owner. It is not a new client command and never resets ingress.
    assert_eq!(owner.use_item_by_definition(actor, use_command, definition_index).await.disposition,
        UseDisposition::Committed);
    assert_returned(h, owner, admitted, hireling).await?;
    Ok(Replay { initial, use_command, use_input, management, hireling })
}

pub(super) async fn replay(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    saved: Replay) -> TestResult {
    let admitted = saved.initial;
    let actor = admitted.runtime_actor.ok_or("replayed native actor")?;
    let moved = owner.runtime.lock().await.house_actor_state(actor,
        admitted.game_session_id).map_err(debug)?;
    assert_eq!((moved.position.x, moved.position.y, moved.position.floor), (32311, 32804, 7),
        "the original lamp command replays after the actor has really moved");
    assert_started_ingress_lost(owner, admitted, management::COMMAND_TYPE_HIRELING_MANAGEMENT,
        &Request::List.encode()).await?;
    let fence = saved.use_command.item_fence.ok_or("original USE fence")?;
    assert_eq!(owner.root.read_hireling_receipt_state(owner.character, fence,
        saved.use_command.command_id, saved.use_input.binding()).await.map_err(debug)?,
        HirelingReceiptRead::Found(Reply::Committed));
    for recorded in saved.management {
        let command = recorded.admitted.continuity.next_command_id;
        let current = recorded.admitted.item_fence.ok_or("original management fence")?;
        let expected = if recorded.request == Request::Projection {
            // Actor handles are a live projection, never an immutable receipt.
            Reply::Projection(Vec::new())
        } else {
            assert_eq!(owner.root.read_hireling_command_receipt(owner.character, current,
                command, recorded.request).await.map_err(debug)?, Some(recorded.answer.clone()),
                "the original framed operation already has its durable answer");
            recorded.answer
        };
        let bytes = owner.hireling_management(recorded.admitted, command, recorded.request)
            .await.ok_or("old management owner retry")?;
        assert_eq!(Reply::decode(&bytes), Some(expected));
        assert_returned(h, owner, admitted, saved.hireling).await?;
    }
    Ok(())
}

async fn assert_returned(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession, hireling: [u8; 16]) -> TestResult {
    assert!(!super::active(h, hireling).await?, "an old USE cannot restore the later returned identity");
    assert_eq!(super::uses(h).await?, 1);
    assert_eq!(super::inbox_count(h).await?, 1);
    let transitions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_hireling_lamp_transitions WHERE hireling_id=encode($1,'hex')::uuid")
        .bind(hireling.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(transitions, 2, "only the original placement and return may change custody");
    assert!(owner.runtime.lock().await.house_hireling_entities(
        admitted.runtime_actor.ok_or("native actor")?, admitted.game_session_id).map_err(debug)?.is_empty());
    Ok(())
}

/// A restarted owner has no retained session stream. Its caller-supplied next
/// counter is not authority to reconstruct one (FND-02 and the approved Q16 rule).
pub(super) async fn assert_started_ingress_lost(owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession, command_type: u32, payload: &[u8]) -> TestResult {
    assert!(admitted.continuity.next_command_id > 1);
    assert!(crate::gameplay_transport::session_ingress::SessionCommandStream::open(
        Some(&owner.states.command_ingress), &admitted).is_none(),
        "lost terminal/high-water truth cannot be recreated from continuity counters");
    let frame = encode_client_command(admitted.continuity.connection_generation,
        &ClientCommandValue { command_id: admitted.continuity.next_command_id, command_type, payload })
        .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_all(&(frame.len() as u32).to_be_bytes()).await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let end = tokio::time::timeout(std::time::Duration::from_secs(15),
        serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS)).await?;
    drop(server);
    let mut responses = Vec::new();
    client.read_to_end(&mut responses).await?;
    assert!(responses.is_empty(), "lost ingress must publish no snapshot or command result");
    let ConnectionEnd::AdmittedThenDisconnected(attempted) = end else {
        return Err(format!("expected closed lost ingress, got {end:?}").into());
    };
    assert_eq!(attempted.continuity, admitted.continuity,
        "refusal must not publish a snapshot or terminal command sequence");
    assert!(!owner.states.command_ingress.resumable(&attempted));
    Ok(())
}

pub(super) async fn enter_interior(owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession, actor: ExactActorRef) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{self as movement, StepDirection};
    let before = owner.runtime.lock().await.house_actor_state(actor,
        admitted.game_session_id).map_err(debug)?;
    assert_eq!((before.position.x, before.position.y, before.position.floor), (32309, 32804, 7),
        "the genuine handoff lands on the source doorway");
    let bytes = movement::encode_step_intent(StepDirection::East);
    let (status, _) = exchange(owner, admitted,
        movement::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, &bytes).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let after = owner.runtime.lock().await.house_actor_state(actor,
        admitted.game_session_id).map_err(debug)?;
    assert_eq!((after.position.x, after.position.y, after.position.floor), (32310, 32804, 7),
        "an actual framed native House STEP moves into the interior");
    Ok(())
}

async fn manage(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    request: Request) -> TestResult<Reply> {
    let (status, payload) = exchange(owner, admitted, management::COMMAND_TYPE_HIRELING_MANAGEMENT,
        &request.encode()).await?;
    assert_eq!(status, CommandStatus::Accepted);
    Reply::decode(&payload).ok_or_else(|| "bounded management reply".into())
}

async fn record_manage(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    records: &mut Vec<RecordedManagement>, request: Request) -> TestResult<Reply> {
    let original = *admitted;
    let answer = manage(owner, admitted, request).await?;
    records.push(RecordedManagement { admitted: original, request, answer: answer.clone() });
    Ok(answer)
}

pub(super) async fn exchange(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    command_type: u32, payload: &[u8]) -> TestResult<(CommandStatus, Vec<u8>)> {
    let command = admitted.continuity.next_command_id;
    let frame = encode_client_command(admitted.continuity.connection_generation,
        &ClientCommandValue { command_id: command, command_type, payload }).map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let reply = async move {
        client.write_all(&(frame.len() as u32).to_be_bytes()).await?;
        client.write_all(&frame).await?;
        loop {
            let n = client.read_u32().await? as usize;
            let mut bytes = vec![0; n];
            client.read_exact(&mut bytes).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, command);
                let answer = (result.status, result.payload.to_vec());
                client.shutdown().await?;
                // Keep the read half alive for the owner's mandatory trailing
                // deltas and updated continuity before it observes the EOF.
                tokio::io::copy(&mut client, &mut tokio::io::sink()).await?;
                return Ok::<_, Box<dyn std::error::Error>>(answer);
            }
        }
    };
    // A queued native STEP needs its connection to stay open until its paced
    // result. Writing EOF before serve_admitted would cancel that real path.
    let input = *admitted;
    let serve = async move {
        let end = serve_admitted(&mut server, input, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let mut serve = std::pin::pin!(serve);
    let mut reply = std::pin::pin!(reply);
    let mut server_end = None;
    let mut client_reply = None;
    let (end, reply) = tokio::time::timeout(std::time::Duration::from_secs(15),
        std::future::poll_fn(|cx| {
            use std::future::Future;
            if server_end.is_none() && let std::task::Poll::Ready(end) = serve.as_mut().poll(cx) {
                server_end = Some(end);
            }
            if client_reply.is_none() && let std::task::Poll::Ready(answer) = reply.as_mut().poll(cx) {
                client_reply = Some(answer);
            }
            if server_end.is_some() && client_reply.is_some() {
                return std::task::Poll::Ready((server_end.take(), client_reply.take()));
            }
            std::task::Poll::Pending
        })).await?;
    let end = end.ok_or("wire server completion")?;
    let reply = reply.ok_or("wire client completion")?;
    if let ConnectionEnd::AdmittedThenDisconnected(current) | ConnectionEnd::AdmittedThenClosed(current, _)
        | ConnectionEnd::AdmittedThenLoggedOut(current) = &end {
        *admitted = *current;
    }
    reply.map_err(|error| format!("wire command {command}/{command_type}: {error}; {end:?}").into())
}
