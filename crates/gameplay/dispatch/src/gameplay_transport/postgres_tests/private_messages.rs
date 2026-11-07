//! S04b: real wire ingress and two live World relay listeners on independent assigned nodes.
use super::chat_world_fixture as world;
use super::fixture::{Harness, debug, id, join_two, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::chat::relay::{RelayKey, RelayKeyRing, channel_name};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::SelectedCapabilities,
    chat_relay::ChatRelay,
    connection::{
        AdmittedSession, ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
    },
    tcp_tls::{read_frame, write_frame},
};
use oteryn_foundation::CancellationToken;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus,
    chat::{self, ChatDisposition, ChatIntent, ChatLine},
    decode_command_result, encode_client_command,
};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::io::AsyncWriteExt;

#[test]
fn private_messages_postgres_cross_node_wire_delivery_replay_and_world_boundary() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().name("chat-world-pm".into()).stack_size(64 * 1024 * 1024)
        .spawn(|| {
            run(async |admin| {
                reset();
                let h = Harness::create_with_admission_seed(admin, "chat_world_pm", true, false).await?;
                prepare_current_session(&h).await?;
                let (second_node, second) = world::admit_second_node(&h).await.map_err(|e| format!("recipient setup: {e}"))?;
                {
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                    let mut first = Dependencies::new()?;
                    let mut fence = super::fixture::fence(1)?.item_fence();
                    fence.character_lease_generation = 2;
                    first.admitted.item_fence = Some(fence);
                    {
                        let mut runtime = first.runtime.lock().await;
                        runtime.initialize_first_entry_position(first.actor).map_err(|e| format!("sender initial position: {e:?}"))?;
                        runtime.bind_attacker_lease(first.actor, first.admitted.game_session_id,
                            crate::foundation::CharacterLease::new(
                                crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?, 2
                            ).map_err(debug)?).map_err(debug)?;
                    }
                    first.admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
                    let owner = first.owner(&h, &authority);
                    let mut target = second.owner(&h, &authority);
                    target.holder = &second_node;
                    assert_ne!(owner.holder.fact().node_id(), target.holder.fact().node_id());
                    assert_ne!(owner.channel_id, target.channel_id);
                    assert_eq!(owner.world_id, target.world_id);
                    first.initialize(&owner).await;
                    second.initialize(&target).await;
                    let mut admitted = first.admitted;
                    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        owner.offered_capabilities(), &[6, 7]).ok_or("chat selection")?;
                    target.chat_admit(second.actor, second.admitted.game_session_id).await;
                    let ring = RelayKeyRing::new(vec![RelayKey::new(1, 0, [19; 32])]).map_err(debug)?;
                    let options = h.root.chat_relay_connect_options().ok_or("relay endpoint")?;
                    let sender_relay = Arc::new(ChatRelay::new(ring.clone(), id(42), options.clone()).map_err(debug)?);
                    let target_relay = Arc::new(ChatRelay::new(ring, id(42), options).map_err(debug)?);
                    owner.states.chat.install_relay(sender_relay.clone());
                    target.states.chat.install_relay(target_relay.clone());
                    let shutdown = CancellationToken::new();
                    let listeners = join_two(
                        sender_relay.run(&owner.states.chat, &shutdown),
                        target_relay.run(&target.states.chat, &shutdown));
                    let scenario = async {
                        while !sender_relay.is_ready() || !target_relay.is_ready() {
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        let channel = channel_name(&crate::durability::guild_chat_fixture::uuid(id(42))).map_err(debug)?;
                        let mut audit = sqlx::postgres::PgListener::connect_with(&h.pool).await?;
                        audit.listen(&channel).await?;
                        command(&owner, &mut admitted, ChatDisposition::Ok).await?;
                        let notification = audit.recv().await?;
                        assert_ne!(notification.payload(), "barrier");
                        // The accepted command and its replay must produce exactly one encrypted notification.
                        sqlx::query("SELECT pg_notify($1,'barrier')").bind(&channel).execute(&h.pool).await?;
                        assert_eq!(audit.recv().await?.payload(), "barrier");
                        let received = loop {
                            if let Some(line) = target.chat_next_bound(second.admitted, &mut 64).await {
                                break line;
                            }
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        };
                        assert!(matches!(received, ChatLine::Private { text, .. } if text == "across two nodes"));
                        target.chat_delivered(second.admitted.game_session_id).await;
                        assert!(target.chat_next_bound(second.admitted, &mut 64).await.is_none());
                        let name = crate::domain::character_name::CharacterName::parse(world::RECIPIENT_NAME).map_err(debug)?;
                        assert_eq!(h.root.read_chat_recipient(&authority, id(42), &name).await.map_err(debug)?, Some(id(world::RECIPIENT)));
                        assert!(h.root.read_chat_recipient(&authority, id(99), &name).await.map_err(debug)?.is_none(),
                            "a name in one World cannot resolve in another World");
                        // The existing admission state owner distinguishes connected recipients from grace.
                        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=1,current_transport_ref=NULL,control_loss_epoch=1,predecessor_generation=current_generation,original_grace_deadline=$1 WHERE game_session_id=encode($2,'hex')::uuid")
                            .bind(world::clock(&h).await? + 60).bind(id(world::RECIPIENT_SESSION).as_slice())
                            .execute(&h.pool).await?;
                        command(&owner, &mut admitted, ChatDisposition::NotOnline).await?;
                        sqlx::query("SELECT pg_notify($1,'barrier')").bind(&channel).execute(&h.pool).await?;
                        assert_eq!(audit.recv().await?.payload(), "barrier");
                        assert!(target.chat_next_bound(second.admitted, &mut 64).await.is_none());
                        Ok::<(), Box<dyn std::error::Error>>(())
                    };
                    let mut listeners = std::pin::pin!(listeners);
                    let mut scenario = std::pin::pin!(scenario);
                    tokio::time::timeout(Duration::from_micros(crate::chat::relay::FRESHNESS_MICROS + crate::chat::relay::RELAY_LEASE_MICROS) + Duration::from_secs(30), std::future::poll_fn(|cx| {
                        let _ = listeners.as_mut().poll(cx);
                        scenario.as_mut().poll(cx)
                    })).await??;
                }
                h.cleanup().await?;
                eprintln!("CHAT-WORLD-PM-PG COMPLETE: two assigned nodes, real wire, encrypted relay, replay, World boundary and offline refusal");
                Ok(())
            }).map_err(|error| error.to_string())
        })?.join().map_err(|_| "chat World PM test panicked")?.map_err(Into::into)
}

async fn command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    expected: ChatDisposition,
) -> TestResult {
    let session = *admitted;
    let (mut server, mut client) = tokio::io::duplex(16_384);
    let serving = async {
        let end = serve_admitted(&mut server, session, owner, IDLE_LIVENESS).await;
        let _ = server.shutdown().await;
        end
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let payload = chat::encode_chat_intent(&ChatIntent::Private {
            recipient_name: world::RECIPIENT_NAME.into(),
            text: "across two nodes".into(),
        })
        .map_err(debug)?;
        let frame = encode_client_command(
            session.continuity.connection_generation,
            &ClientCommandValue {
                command_id: session.continuity.next_command_id,
                command_type: chat::COMMAND_TYPE_CHAT_INTENT,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        for status in [CommandStatus::Accepted, CommandStatus::DuplicateReplay] {
            write_frame(&mut client, &frame).await?;
            loop {
                let bytes = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                if envelope.message_type() != MessageType::CommandResult {
                    continue;
                }
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, session.continuity.next_command_id);
                assert_eq!(result.status, status);
                assert_eq!(
                    chat::decode_chat_intent_result(result.payload)
                        .map_err(debug)?
                        .disposition,
                    expected
                );
                break;
            }
        }
        client.shutdown().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (end, result) =
        tokio::time::timeout(Duration::from_secs(10), join_two(serving, driving)).await?;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(current) => *admitted = current,
        other => return Err(format!("private message connection end: {other:?}").into()),
    }
    result
}
