//! Real native Base12816 USE_WITH after an actual participant portal visit.
use super::{
    TestResult,
    fixture::{Harness, debug},
};
use crate::durability::seasonal_pumpkin::PumpkinRewardVisit;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted},
};
use oteryn_protocol_oteryn::world_object::{
    self, MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

thread_local! { static CUES: std::cell::RefCell<Vec<(u32,Vec<u8>)>> = const { std::cell::RefCell::new(Vec::new()) }; }
thread_local! { static CLAIM_FRAMES: std::cell::RefCell<Vec<serde_json::Value>> = const { std::cell::RefCell::new(Vec::new()) }; }
pub(super) fn take_claim_frames() -> Vec<serde_json::Value> {
    CLAIM_FRAMES.with(|c| std::mem::take(&mut *c.borrow_mut()))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn take_cues() -> Vec<(u32, Vec<u8>)> {
    CUES.with(|c| std::mem::take(&mut *c.borrow_mut()))
}
fn check_cues(cues: &[(u32, Vec<u8>)], expected: usize) -> TestResult {
    let mut speech = 0;
    let mut effects = 0;
    for (domain, payload) in cues {
        if *domain == oteryn_protocol_oteryn::chat::STATE_DOMAIN_CHAT {
            let line = oteryn_protocol_oteryn::chat::decode_chat_line(payload).map_err(debug)?;
            if let oteryn_protocol_oteryn::chat::ChatLine::Local {
                mode,
                text,
                position,
                ..
            } = line
                && text == "Happy Halloween!"
            {
                assert_eq!(
                    mode,
                    oteryn_protocol_oteryn::chat::ChatSpeechMode::MonsterSay
                );
                assert_eq!((position.x, position.y, position.floor), (33317, 32415, 15));
                speech += 1;
            }
        }
        if *domain == oteryn_protocol_oteryn::presentation::STATE_DOMAIN_ITEM_EFFECTS {
            for event in oteryn_protocol_oteryn::presentation::decode_item_effect_batch(payload)
                .map_err(debug)?
            {
                if let oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
                    effect_id: 28,
                    position,
                    ..
                } = event.kind
                {
                    assert_eq!((position.x, position.y, position.floor), (33317, 32415, 15));
                    effects += 1;
                }
            }
        }
    }
    assert_eq!(
        (speech, effects),
        (expected, expected),
        "actual native publisher emits exactly fresh source speech/effect, never replay"
    );
    Ok(())
}

pub(super) async fn qualify(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    visit: &PumpkinRewardVisit,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("current reward actor")?;
    super::item_look::seed_main_backpack(h, &[("oteryn:item.tibia.i5908", 1)]).await?;
    for _ in 0..2 {
        let bytes = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
            oteryn_protocol_oteryn::world_spatial::StepDirection::North,
        );
        let (status, _) = exchange(
            owner,
            &mut admitted,
            oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            &bytes,
            None,
        )
        .await?;
        assert_eq!(
            status,
            CommandStatus::Accepted,
            "actual paced North approaches static body"
        );
    }
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!((at.x, at.y, at.floor), (33317, 32415, 15));
    let reference = {
        let runtime = owner.runtime.lock().await;
        let index = crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime,
            owner.active_generation,
        )
        .ok_or("native Item index")?;
        let key = "oteryn:item.tibia.i5908";
        index
            .definition_ref("Item", key, index.revision(key).ok_or("knife revision")?)
            .ok_or("native knife definition")?
    };
    let (target, before) = body(owner)?;
    let bytes = world_object::encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(reference),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
        use_with_position: None,
    })
    .map_err(debug)?;
    let _ = take_cues();
    let _ = take_claim_frames();
    let original_command = admitted.continuity.next_command_id;
    let (status, reply) = exchange(
        owner,
        &mut admitted,
        world_object::COMMAND_TYPE_USE_INTENT,
        &bytes,
        None,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed,
        "current participant must receive native reward from actual Base12816"
    );
    let fresh_cues = take_cues();
    let fresh_claim_frames = take_claim_frames();
    check_cues(&fresh_cues, 1)?;
    let (_, after) = body(owner)?;
    assert_eq!(before, after, "static body remains byte-for-byte unchanged");
    let source = (visit.source_session, visit.draw_nonce(), visit.death);
    let expected = oteryn_gameplay_items::pumpkin_skinning::select(5908, 12816, |domain, upper| {
        oteryn_gameplay_items::reward_bag::reward_bag_bounded_draw(
            source.0, source.1, source.2, domain, upper,
        )
    })
    .ok_or("closed source reward")?;
    let actual:(i64,i64,String,i64)=sqlx::query_as("SELECT p.source_item_id,p.quantity,i.definition_production_key,i.quantity FROM game_pumpkin_skinning_receipts p JOIN game_reward_claim_mint_receipts r USING(transaction_id) JOIN game_item_instances i USING(item_instance_id) WHERE p.game_session_id=encode($1,'hex')::uuid AND p.command_id=$2::text::numeric")
 .bind(admitted.game_session_id.as_bytes().as_slice()).bind(original_command.to_string()).fetch_one(&h.pool).await?;
    assert_eq!(
        actual,
        (
            i64::from(expected.item_id),
            i64::from(expected.count),
            format!("oteryn:item.tibia.i{}", expected.item_id),
            i64::from(expected.count)
        )
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i12816'").fetch_one(&h.pool).await?,0,"no fabricated second corpse or reward-body MINT");
    let (status, replay) = exchange(
        owner,
        &mut admitted,
        world_object::COMMAND_TYPE_USE_INTENT,
        &bytes,
        Some(original_command),
    )
    .await?;
    assert_eq!(status, CommandStatus::DuplicateReplay);
    assert_eq!(
        world_object::decode_use_result(&replay).map_err(debug)?,
        UseDisposition::Committed
    );
    check_cues(&take_cues(), 0)?;
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    let (_, again) = exchange(
        owner,
        &mut admitted,
        world_object::COMMAND_TYPE_USE_INTENT,
        &bytes,
        None,
    )
    .await?;
    assert_eq!(
        world_object::decode_use_result(&again).map_err(debug)?,
        UseDisposition::Exhausted,
        "unchanged static body cannot bypass personal4h cooldown"
    );
    check_cues(&take_cues(), 0)?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_pumpkin_skinning_receipts")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/mutated_presents'").fetch_one(&h.pool).await?,1);
    let fixed:(i64,i64)=sqlx::query_as("SELECT first_entry_ms,deadline_ms FROM game_pumpkin_reward_visits WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND death_id=encode($3,'hex')::uuid")
 .bind(admitted.item_fence.ok_or("current fence")?.character_id.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(visit.death.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        fixed,
        (visit.first_entry_ms, visit.deadline_ms),
        "claim/replay keep the original visit deadline"
    );
    if let Some(path) = std::env::var_os("OTERYN_SRV7_PUMPKIN_CAPTURE") {
        let mut capture: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        let frames:Vec<_>=fresh_cues.iter().map(|(domain,payload)|serde_json::json!({"domain":domain,"payload_hex":payload.iter().map(|b|format!("{b:02x}")).collect::<String>()})).collect();
        let snapshots: Vec<_> = fresh_claim_frames
            .iter()
            .filter(|f| f["kind"] == "snapshot")
            .cloned()
            .collect();
        let deltas: Vec<_> = fresh_claim_frames
            .iter()
            .filter(|f| f["kind"] == "delta")
            .cloned()
            .collect();
        let wire_frames: Vec<_> = fresh_claim_frames
            .iter()
            .filter(|f| f["kind"] == "wire")
            .cloned()
            .collect();
        assert!(
            snapshots.iter().any(|f| f["domain"]
                == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY),
            "actual before-claim native inventory snapshot"
        );
        assert!(
            snapshots.iter().any(|f| f["domain"] == 17),
            "actual reward-room native map snapshot"
        );
        assert!(
            snapshots
                .iter()
                .any(|f| f["domain"] == oteryn_protocol_oteryn::chat::STATE_DOMAIN_CHAT),
            "actual native chat bootstrap"
        );
        assert!(
            deltas.iter().any(|f| f["domain"]
                == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY),
            "actual after-claim inventory delta"
        );
        let reward_definition_ref = {
            let runtime = owner.runtime.lock().await;
            let index = crate::gameplay_transport::item_ref_admission::pinned_index(
                &runtime,
                owner.active_generation,
            )
            .ok_or("actual claimed Item index")?;
            index
                .definition_ref(
                    "Item",
                    &actual.2,
                    index.revision(&actual.2).ok_or("claim Item revision")?,
                )
                .ok_or("claim Item definition ref")?
                .get()
        };
        capture["claim_presentation"] = serde_json::json!({"frames":frames,"snapshots":snapshots,"deltas":deltas,"wire_frames":wire_frames,"selected_capabilities":admitted.continuity.selected_capabilities.as_slice(),"observer_runtime":{"x":at.x,"y":at.y,"floor":at.floor},"observer_native":{"x":at.x,"y":at.y,"floor":-i32::from(at.floor)},"observer_actor_hex":hex(&actor.placement_identity()),"knife_definition_ref":reference.get(),"use_result_hex":hex(&reply),"original_command_id":original_command,"claimed_reward":{"item_definition_ref":reward_definition_ref,"count":actual.1,"source_item_id":actual.0},"fresh_speech":"Happy Halloween!","effect_id":28,"position":[33317,32415,15],"replay_cues":0,"producer_test":"srv7_pumpkin_actual_fight_portal_visit_and_static_reward_body_use"});
        std::fs::write(path, serde_json::to_vec_pretty(&capture)?)?;
    }
    eprintln!(
        "SRV7_PUMPKIN_STATIC_BODY_NATIVE_POSTGRES_EXECUTED real_fight=true real_portal=true real_base12816=true framed_use_with=true native_mint=true replay=true personal_cooldown=true no_body_mint=true"
    );
    Ok(())
}
fn body(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<(WorldObjectTarget, String)> {
    use crate::map::view::MapFacts;
    let map = owner.bundle_map().ok_or("actual Pumpkin B3 map")?;
    let facts = map.facts.pinned();
    let pos = crate::map::overlay::TilePos {
        x: 33317,
        y: 32414,
        floor: -15,
    };
    let tile = crate::map::view::compose_tile(&map.overlay, &facts, pos)
        .map_err(debug)?
        .ok_or("static body source tile")?;
    let entry = tile
        .entries
        .iter()
        .find(|e| e.facts.appearance_id == 12816)
        .ok_or("actual12816 base entry")?;
    let crate::map::view::MapEntryKey::Base(key) = entry.source.key() else {
        return Err("static reward body must be actual B3 Base".into());
    };
    assert!(!entry.facts.pickupable, "official12816 is immovable");
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend(key.to_be_bytes());
    Ok((
        WorldObjectTarget {
            placement,
            expected_revision: facts.object_revision(key),
        },
        format!("{entry:?}"),
    ))
}
pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command_type: u32,
    payload: &[u8],
    replay: Option<u64>,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    exchange_observe(
        owner,
        admitted,
        command_type,
        payload,
        replay,
        crate::gameplay_transport::chat_intent::CHAT_REFRESH * 2,
    )
    .await
}
pub(super) async fn exchange_observe(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command_type: u32,
    payload: &[u8],
    replay: Option<u64>,
    observation_window: std::time::Duration,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let command = replay.unwrap_or(admitted.continuity.next_command_id);
    let frame = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id: command,
            command_type,
            payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let reply = async move {
        client
            .write_all(&(frame.len() as u32).to_be_bytes())
            .await?;
        client.write_all(&frame).await?;
        let mut answer = None;
        let mut close_at = None;
        loop {
            // AsyncReadExt::read is cancel-safe; retain any partial frame prefix
            // when the chat observation deadline expires.
            let mut prefix = [0_u8; 4];
            let mut used = 0;
            while used < prefix.len() {
                let read = if let Some(deadline) = close_at {
                    match tokio::time::timeout_at(deadline, client.read(&mut prefix[used..])).await
                    {
                        Ok(read) => read,
                        Err(_) => {
                            client.shutdown().await?;
                            close_at = None;
                            continue;
                        }
                    }
                } else {
                    client.read(&mut prefix[used..]).await
                };
                match read {
                    Ok(0) if used == 0 => {
                        return Ok::<_, Box<dyn std::error::Error>>(
                            answer.ok_or("missing command result")?,
                        );
                    }
                    Ok(0) => return Err("partial server frame prefix".into()),
                    Ok(n) => used += n,
                    Err(error) => return Err(error.into()),
                }
            }
            let n = u32::from_be_bytes(prefix) as usize;
            let mut bytes = vec![0; n];
            client.read_exact(&mut bytes).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            CLAIM_FRAMES.with(|capture|capture.borrow_mut().push(serde_json::json!({"kind":"wire","message_type":format!("{:?}",envelope.message_type()),"server_sequence":envelope.server_sequence(),"envelope_hex":hex(&bytes),"payload_hex":hex(envelope.payload())})));
            if envelope.message_type() == MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                        .map_err(debug)?;
                for domain in domains {
                    CLAIM_FRAMES.with(|capture|capture.borrow_mut().push(serde_json::json!({"kind":"snapshot","domain":domain.domain_id,"revision":domain.revision,"snapshot_type":domain.snapshot_type,"payload_hex":hex(domain.payload),"server_sequence":envelope.server_sequence()})));
                }
            }
            if envelope.message_type() == MessageType::StateDelta {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                    .map_err(debug)?;
                CLAIM_FRAMES.with(|capture|capture.borrow_mut().push(serde_json::json!({"kind":"delta","domain":delta.domain_id,"base_revision":delta.base_revision,"new_revision":delta.new_revision,"delta_type":delta.delta_type,"payload_hex":hex(delta.payload),"state_delta_hex":hex(envelope.payload()),"server_sequence":envelope.server_sequence()})));
                if matches!(
                    delta.domain_id,
                    oteryn_protocol_oteryn::chat::STATE_DOMAIN_CHAT
                        | oteryn_protocol_oteryn::presentation::STATE_DOMAIN_ITEM_EFFECTS
                ) {
                    CUES.with(|c| {
                        c.borrow_mut()
                            .push((delta.domain_id, delta.payload.to_vec()))
                    });
                }
            }
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, command);
                answer = Some((result.status, result.payload.to_vec()));
                // Ordinary CHAT uses its existing250ms cadence rather than the
                // mandatory item-effect flush. Keep the genuine connection alive
                // for two ticks; the strict cue assertions still fail if it emits none.
                close_at = Some(tokio::time::Instant::now() + observation_window);
                // Continue decoding every actual publisher frame through EOF.
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
    let (end, reply) = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        std::future::poll_fn(|cx| {
            use std::future::Future;
            if server_end.is_none()
                && let std::task::Poll::Ready(end) = serve.as_mut().poll(cx)
            {
                server_end = Some(end);
            }
            if client_reply.is_none()
                && let std::task::Poll::Ready(answer) = reply.as_mut().poll(cx)
            {
                client_reply = Some(answer);
            }
            if server_end.is_some() && client_reply.is_some() {
                return std::task::Poll::Ready((server_end.take(), client_reply.take()));
            }
            std::task::Poll::Pending
        }),
    )
    .await?;
    let end = end.ok_or("wire server completion")?;
    let reply = reply.ok_or("wire client completion")?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _)
    | ConnectionEnd::AdmittedThenLoggedOut(current) = &end
    {
        *admitted = *current;
    }
    reply.map_err(|error| format!("wire command {command}/{command_type}: {error}; {end:?}").into())
}
