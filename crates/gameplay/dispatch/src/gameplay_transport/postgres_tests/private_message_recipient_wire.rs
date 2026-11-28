use super::chat_world_fixture::{RECIPIENT_NAME, admit_second_node};
use super::cyclopedia_fixtures::support::exchange;
use super::fixture::{Harness, debug, id, join_two};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::chat::relay::{FRESHNESS_MICROS, RELAY_LEASE_MICROS, RelayKey, RelayKeyRing};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::chat_relay::ChatRelay;
use crate::gameplay_transport::connection::{FirstEntryOutcome, IDLE_LIVENESS, serve_admitted};
use crate::gameplay_transport::tcp_tls::read_frame;
use oteryn_foundation::CancellationToken;
use oteryn_protocol_oteryn::chat::{self, ChatDisposition, ChatIntent, ChatLine};
use oteryn_protocol_oteryn::{
    CommandStatus, MessageType, decode_state_delta, decode_wire_envelope,
};
use std::{sync::Arc, time::Duration};
use tokio::io::AsyncWriteExt;

#[test]
fn private_message_reaches_idle_same_world_recipient_wire_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_async(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "pm_recipient_wire", true, false).await?;
        let result: TestResult = async {
            prepare_current_session(&h).await?;
            let (peer_node, peer) = admit_second_node(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let sender = Dependencies::new()?;
            sender.runtime.lock().await.initialize_first_entry_position(sender.actor).map_err(debug)?;
            let owner = sender.owner(&h, &authority);
            sender.initialize(&owner).await;
            let mut receiver = peer.owner(&h, &authority);
            receiver.holder = &peer_node;
            peer.initialize(&receiver).await;
            assert_eq!(owner.world_id, receiver.world_id);
            assert_ne!(owner.channel_id, receiver.channel_id);

            let selected = SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[chat::CAPABILITY_CHAT_V1],
            )
            .ok_or("chat selection")?;
            let mut outgoing = sender.admitted;
            outgoing.first_entry = FirstEntryOutcome::Positioned;
            outgoing.continuity.selected_capabilities = selected;
            outgoing.item_fence = Some(
                super::death_ml_loss_composed_fixtures::current_fence(&owner, outgoing.game_session_id)
                    .await?
                    .item_fence(),
            );
            let fence = outgoing.item_fence.ok_or("sender fence")?;
            sender
                .runtime
                .lock()
                .await
                .bind_attacker_lease(
                    sender.actor,
                    outgoing.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            let mut incoming = peer.admitted;
            incoming.continuity.selected_capabilities = selected;

            let ring = RelayKeyRing::new(vec![RelayKey::new(1, 0, [19; 32])]).map_err(debug)?;
            let options = h.root.chat_relay_connect_options().ok_or("relay endpoint")?;
            let send_relay = Arc::new(ChatRelay::new(ring.clone(), id(42), options.clone()).map_err(debug)?);
            let receive_relay = Arc::new(ChatRelay::new(ring, id(42), options).map_err(debug)?);
            owner.states.chat.install_relay(send_relay.clone());
            receiver.states.chat.install_relay(receive_relay.clone());

            let sender_name: String = sqlx::query_scalar(
                "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(id(41).as_slice())
            .fetch_one(&h.pool)
            .await?;
            let shutdown = CancellationToken::new();
            let scenario = async {
                // One listener can start against the other's stored replay lease.
                tokio::time::timeout(
                    Duration::from_micros(FRESHNESS_MICROS + RELAY_LEASE_MICROS) + Duration::from_secs(10),
                    async {
                        while !send_relay.is_ready() || !receive_relay.is_ready() {
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                    },
                )
                .await?;
                let (mut server, client) = tokio::io::duplex(1 << 20);
                let recipient_owner = &receiver;
                let serving = async move {
                    serve_admitted(&mut server, incoming, recipient_owner, IDLE_LIVENESS).await
                };
                let sender_owner = &owner;
                let driving = async move {
                    let mut client = client;
                    loop {
                        let frame = read_frame(&mut client).await?;
                        if decode_wire_envelope(&frame).map_err(debug)?.message_type() == MessageType::SnapshotCommit {
                            break;
                        }
                    }
                    let text = "a private line to an idle recipient";
                    let request = chat::encode_chat_intent(&ChatIntent::Private {
                        recipient_name: RECIPIENT_NAME.into(),
                        text: text.into(),
                    })
                    .map_err(debug)?;
                    let command = outgoing.continuity.next_command_id;
                    let (status, response) = exchange(
                        sender_owner, &mut outgoing, command, chat::COMMAND_TYPE_CHAT_INTENT, &request,
                    )
                    .await?;
                    assert_eq!(status, CommandStatus::Accepted);
                    assert_eq!(chat::decode_chat_intent_result(&response).map_err(debug)?.disposition, ChatDisposition::Ok);
                    loop {
                        let frame = read_frame(&mut client).await?;
                        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                        if envelope.message_type() != MessageType::StateDelta {
                            continue;
                        }
                        let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                        if delta.domain_id == chat::STATE_DOMAIN_CHAT
                            && delta.delta_type == chat::DELTA_TYPE_CHAT_LINE_V1
                            && let ChatLine::Private { speaker_name, text: received } =
                                chat::decode_chat_line(delta.payload).map_err(debug)?
                        {
                            assert_eq!(speaker_name, sender_name);
                            assert_eq!(received, text);
                            break;
                        }
                    }
                    client.shutdown().await?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                };
                let (ended, delivered) =
                    tokio::time::timeout(Duration::from_secs(10), join_two(serving, driving)).await?;
                delivered.map_err(|error| format!("recipient wire ended {ended:?}: {error}"))?;
                Ok::<(), Box<dyn std::error::Error>>(())
            };
            tokio::select! {
                result = scenario => result?,
                () = send_relay.run(&owner.states.chat, &shutdown) => return Err("sender relay exited".into()),
                () = receive_relay.run(&receiver.states.chat, &shutdown) => return Err("recipient relay exited".into()),
            }
            eprintln!("private_message_reaches_idle_same_world_recipient_wire_postgres POSTGRES_CASE_COMPLETED");
            Ok(())
        }.await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}
