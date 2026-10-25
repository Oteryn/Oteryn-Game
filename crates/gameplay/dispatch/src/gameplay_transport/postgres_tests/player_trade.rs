//! TRADE-WIRE-1: two independently admitted sessions, real ingress and atomic native writer.
use super::{Dependencies, TestResult, fixture::join_two};
use crate::bestiary_postgres_harness::{self as fixture, CHANNEL, Harness, WORLD, debug, id};
use crate::foundation::{ChannelId, ChannelRuntimeV1, MessageType, WorldId};
use crate::gameplay_transport::{
    SeamRooms,
    capabilities::SelectedCapabilities,
    connection::{
        AdmittedSession, ControllerBinding, FirstEntryOutcome, FreshAdmissionAuthority,
        IDLE_LIVENESS, SessionContinuity, serve_admitted,
    },
    tcp_tls::{read_frame, write_frame},
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, decode_state_delta,
    decode_wire_envelope, encode_client_command, item_view,
    player_trade::{self as wire, Intent, ResultCode},
    world_spatial_entities::EntityRef,
};
use tokio::{io::AsyncWriteExt, sync::Mutex};

#[test]
fn player_trade_postgres_two_real_sessions_atomic_replay_and_cancellation() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            fixture::runtime().map_err(debug)?.block_on(async move {
                let h = Harness::create(admin, "player_trade_wire", true)
                    .await
                    .map_err(debug)?;
                let result = scenario(&h).await.map_err(|e| e.to_string());
                h.cleanup().await.map_err(debug)?;
                result
            })
        })?
        .join()
        .map_err(|_| "trade wire thread panicked")??;
    Ok(())
}
async fn seed(h: &Harness, character: [u8; 16], bag: u8, leaf: u8, key: &str) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(character.as_slice()).execute(&mut *tx).await?;
    for (item, definition, quantity) in [(bag, "oteryn:item.tibia.i2854", 1), (leaf, key, 3)] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',$4,1,encode($5,'hex')::uuid)")
            .bind(id(item).as_slice()).bind(id(WORLD).as_slice()).bind(definition).bind(quantity).bind(id(120).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(character.as_slice()).bind(id(bag).as_slice()).bind(id(WORLD).as_slice()).bind(id(bag+20).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(character.as_slice()).bind(id(WORLD).as_slice()).bind(id(bag).as_slice()).bind(id(leaf).as_slice()).bind(id(leaf+20).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn scenario(h: &Harness) -> TestResult {
    eprintln!("TRADE-WIRE-1: PostgreSQL harness started; two native Fresh admissions");
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut currents = Vec::new();
    for (character, account, session) in [(60, 63, 70), (61, 64, 71)] {
        currents.push(
            crate::world_party_runtime_postgres_cases::committed_fresh_player(
                h, character, account, session, now,
            )
            .await?,
        );
    }

    seed(
        h,
        *currents[0].commit().character_id().as_bytes(),
        90,
        91,
        "oteryn:item.tibia.i3607",
    )
    .await?;
    seed(
        h,
        *currents[1].commit().character_id().as_bytes(),
        92,
        93,
        "oteryn:item.tibia.i3604",
    )
    .await?;

    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;

    for current in &currents {
        use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
        use crate::durability::character_progression::{
            CurrentCharacterGameplayFence, ProgressionInitializationRequest,
        };
        use oteryn_simulation_determinism::{ExactI64, RoundingMode};
        let item = crate::gameplay_transport::item_fence_of(*current).ok_or("item fence")?;
        let context = fixture::context();
        let fence = CurrentCharacterGameplayFence {
            character_id: item.character_id,
            game_session_id: item.game_session_id,
            connection_generation: item.connection_generation,
            character_lease_generation: item.character_lease_generation,
            runtime_scope: item.runtime_scope,
            scope_ownership_generation: item.scope_ownership_generation,
            expected_character_revision: crate::domain::CharacterRevision::new(1).map_err(debug)?,
        };
        h.root
            .initialize_character_progression(
                &recovery,
                &h.node,
                fence,
                ProgressionInitializationRequest {
                    context: context.clone(),
                    policy_revision: "policy-1".into(),
                    reward_revision: "reward-1".into(),
                    policy: FiniteProgressionPolicy {
                        context,
                        policy_revision: "policy-1".into(),
                        reward_revision: "reward-1".into(),
                        death_policy_revision: "death-1".into(),
                        declared_difference_revision: "declaration-1".into(),
                        thresholds: [
                            LevelThreshold {
                                level: 1,
                                minimum_experience: ExactI64::new(0),
                            },
                            LevelThreshold {
                                level: 2,
                                minimum_experience: ExactI64::new(100),
                            },
                        ],
                        terminal_exclusive_experience: ExactI64::new(200),
                        death_loss_numerator: 1,
                        death_loss_denominator: 10,
                        death_loss_rounding: RoundingMode::Floor,
                    },
                },
            )
            .await
            .map_err(debug)?;
    }
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            world,
            Some(include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        );
    let active = controller.active().ok_or("active")?;
    let frame_digest = pin.frame_binding().digest();
    let room = pin.qualified_room().clone();
    let (channel_pin, _, _, _) = pin.into_channel_parts();
    let deps = Dependencies::with_content(
        channel_pin.clone(),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        h.node.fact().node_id(),
        h.node.fact().registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        5,
        channel_pin,
    )
    .map_err(debug)?;
    let mut admitted = Vec::new();
    for (i, current) in currents.iter().enumerate() {
        let session = current.current_game_session_id();
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        admitted.push(AdmittedSession {
            game_session_id: session,
            world_id: world,
            channel_id: channel,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller: Some(ControllerBinding {
                account_id: id(63 + u8::try_from(i)?),
                transport: current.current_transport().ok_or("native transport")?,
            }),
            continuity: SessionContinuity::FRESH,
            item_fence: crate::gameplay_transport::item_fence_of(*current),
        });
    }
    let targets: Vec<_> = runtime
        .visible_entities()
        .players
        .iter()
        .map(|p| EntityRef {
            identity: p.actor.placement_identity(),
            generation: p.generation,
        })
        .collect();
    let runtime = Mutex::new(runtime);
    let mut owner = crate::gameplay_transport::composed_test_owner::ComposedTestOwner { root: &h.root, character: &recovery, holder: &h.node, evidence: &deps.evidence, world_id: world, channel_id: channel,
        required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(&[], currents[0].current_runtime_scope(), 1, "policy-1", crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, Some(active))).map_err(debug)?,
        runtime: &runtime, room: &deps.room, door: &deps.door, spells: &deps.spells, achievements: &deps.achievements, imported_charms: &deps.charms }.build();
    owner.active_generation = Some(active);
    owner.rooms = SeamRooms::new(Some(&deps.room), Some(&deps.room));
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&h.pool).await?;
    assert!(
        owner.ensure_source_map_initialized().await,
        "real source-map initialization"
    );
    for a in &mut admitted {
        a.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 21, 60])
                .ok_or("selected")?;
        assert!(a.continuity.selected_capabilities.contains(60));
    }

    let (mut server_a, client_a) = tokio::io::duplex(1 << 20);
    let (mut server_b, client_b) = tokio::io::duplex(1 << 20);
    let (start_b, ready_b) = tokio::sync::oneshot::channel();
    let owner_ref = &owner;
    let a_admitted = admitted[0];
    let b_admitted = admitted[1];
    let serving = join_two(
        async move { serve_admitted(&mut server_a, a_admitted, owner_ref, IDLE_LIVENESS).await },
        async move {
            let _ = ready_b.await;
            serve_admitted(&mut server_b, b_admitted, owner_ref, IDLE_LIVENESS).await
        },
    );
    let driving = async {
        let mut a = Peer::open(client_a).await?;
        start_b.send(()).map_err(|_| "start second session")?;
        let mut b = Peer::open(client_b).await?;

        let offer_a = Intent::Offer {
            partner: targets[1],
            item: 2,
        };
        let offer_b = Intent::Offer {
            partner: targets[0],
            item: 2,
        };
        assert_eq!(a.trade(offer_a).await?, ResultCode::Offered.payload());
        let (_, first, _) = owner
            .trade_observation(admitted[1].game_session_id)
            .await
            .ok_or("partner offer state")?;
        assert!(first.counter.is_none());
        assert!(first.message.ends_with(" wants to trade with you."));
        assert_eq!(b.trade(offer_b).await?, ResultCode::Offered.payload());
        let (_, state, _) = owner
            .trade_observation(admitted[0].game_session_id)
            .await
            .ok_or("ready")?;
        assert_eq!(state.phase, 2);
        assert!(state.message.is_empty());
        assert!(state.own.is_some() && state.counter.is_some());
        let trade_id = state.trade_id.ok_or("trade id")?;
        assert_eq!(
            a.trade(Intent::Look {
                trade_id,
                counter_offer: true
            })
            .await?,
            ResultCode::Offered.payload()
        );
        assert_eq!(
            b.trade(Intent::Look {
                trade_id,
                counter_offer: true
            })
            .await?,
            ResultCode::Offered.payload()
        );
        let looked_a = owner
            .trade_observation(admitted[0].game_session_id)
            .await
            .ok_or("own inspection")?
            .1;
        let looked_b = owner
            .trade_observation(admitted[1].game_session_id)
            .await
            .ok_or("partner inspection")?
            .1;
        assert!(!looked_a.look.is_empty() && !looked_b.look.is_empty());
        a.trade_state(&looked_a).await?;
        b.trade_state(&looked_b).await?;
        let (accepted_a, accepted_b) = join_two(
            a.trade(Intent::Accept { trade_id }),
            b.trade(Intent::Accept { trade_id }),
        )
        .await;
        let accepted_a = accepted_a?;
        let accepted_b = accepted_b?;
        assert!(
            (accepted_a == ResultCode::Accepted.payload()
                && accepted_b == ResultCode::Completed.payload())
                || (accepted_b == ResultCode::Accepted.payload()
                    && accepted_a == ResultCode::Completed.payload())
        );
        let completing_peer = if accepted_a == ResultCode::Completed.payload() {
            &mut a
        } else {
            &mut b
        };
        let replay = completing_peer
            .send(completing_peer.next - 1, 78, &[0xff])
            .await?;
        assert_eq!(replay.0, CommandStatus::DuplicateReplay);
        assert_eq!(replay.1, ResultCode::Completed.payload());
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let expected_a = index
            .definition_ref("Item", "oteryn:item.tibia.i3604", "definition-r1")
            .ok_or("a item")?;
        let expected_b = index
            .definition_ref("Item", "oteryn:item.tibia.i3607", "definition-r1")
            .ok_or("b item")?;
        a.inventory(expected_a).await?;
        b.inventory(expected_b).await?;
        let owners: Vec<(Vec<u8>,Vec<u8>)> = sqlx::query_as("SELECT uuid_send(item_instance_id),uuid_send(character_id) FROM game_item_container_entries WHERE item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid) ORDER BY item_instance_id")
            .bind(id(91).as_slice()).bind(id(93).as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(
            owners,
            vec![
                (
                    id(91).to_vec(),
                    currents[1].commit().character_id().as_bytes().to_vec()
                ),
                (
                    id(93).to_vec(),
                    currents[0].commit().character_id().as_bytes().to_vec()
                )
            ]
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_player_trades")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        // A fresh dialog uses the new inventory handle. Cancellation works from either side.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let handle_a = a.last_inventory.as_ref().ok_or("a inventory")?.entries[0]
            .handle
            .get();
        let handle_b = b.last_inventory.as_ref().ok_or("b inventory")?.entries[0]
            .handle
            .get();
        let offer_a = Intent::Offer {
            partner: targets[1],
            item: handle_a,
        };
        let offer_b = Intent::Offer {
            partner: targets[0],
            item: handle_b,
        };
        for cancel_by_a in [true, false] {
            assert_eq!(a.trade(offer_a).await?, ResultCode::Offered.payload());
            assert!(
                owner
                    .trade_observation(admitted[0].game_session_id)
                    .await
                    .ok_or("fresh offer")?
                    .1
                    .message
                    .is_empty()
            );
            assert_eq!(
                if cancel_by_a {
                    a.trade(Intent::Cancel).await?
                } else {
                    b.trade(Intent::Cancel).await?
                },
                ResultCode::Cancelled.payload()
            );
        }
        assert_eq!(a.trade(offer_a).await?, ResultCode::Offered.payload());
        assert_eq!(b.trade(offer_b).await?, ResultCode::Offered.payload());
        let pair = owner
            .trade_observation(admitted[0].game_session_id)
            .await
            .ok_or("pair")?
            .1
            .trade_id
            .ok_or("pair id")?;
        assert_eq!(
            a.trade(Intent::Accept { trade_id: pair }).await?,
            ResultCode::Accepted.payload()
        );
        // The accepted rule cancels a changed offer, retiring both accepts and the old id.
        assert_eq!(b.trade(offer_b).await?, ResultCode::Stale.payload());
        assert_eq!(
            a.trade(Intent::Accept { trade_id: pair }).await?,
            ResultCode::Stale.payload()
        );
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert_eq!(a.trade(offer_a).await?, ResultCode::Offered.payload());
        assert_eq!(
            owner
                .trade_observation(admitted[0].game_session_id)
                .await
                .ok_or("reserved move source")?
                .1
                .phase,
            1
        );
        let payload = item_view::encode_item_move_intent(&item_view::ItemMoveIntent {
            source: std::num::NonZeroU64::new(handle_a).ok_or("handle")?,
            destination: item_view::ItemMoveDestination::MainBackpack,
        })
        .map_err(debug)?;
        let move_result = a.command(9, &payload).await?;
        assert_ne!(
            item_view::decode_item_move_result(&move_result).map_err(debug)?,
            item_view::ItemMoveOutcome::Moved
        );
        assert_eq!(a.trade(offer_a).await?, ResultCode::Offered.payload());
        let payload = oteryn_protocol_oteryn::world_object::encode_item_use_intent(
            &oteryn_protocol_oteryn::world_object::UseIntent {
                target: oteryn_protocol_oteryn::world_object::UseTarget::Item(
                    std::num::NonZeroU64::new(handle_a).ok_or("offered handle")?,
                ),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            },
        )
        .map_err(debug)?;
        let used = a.send(a.next, 2, &payload).await?;
        a.next += 1;
        assert_eq!(used.0, CommandStatus::Rejected);
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&used.1).map_err(debug)?,
            oteryn_protocol_oteryn::world_object::UseDisposition::Rejected
        );
        assert_eq!(a.trade(offer_a).await?, ResultCode::Offered.payload());
        let actor = admitted[1].runtime_actor.ok_or("actor")?;
        let original = runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        for _ in 0..3 {
            let mut runtime = runtime.lock().await;
            let p = runtime.read_actor_position(actor).map_err(debug)?;
            runtime
                .borrow_movement_position()
                .commit_cardinal(
                    p,
                    crate::foundation::MovementLocalPosition {
                        x: p.position().x + 1,
                        ..p.position()
                    },
                )
                .map_err(debug)?;
        }
        let walked_away = owner
            .trade_observation(admitted[0].game_session_id)
            .await
            .ok_or("walk away cancellation")?
            .1;
        assert_eq!((walked_away.phase, walked_away.reason), (4, 2));
        assert_eq!(a.trade(offer_a).await?, ResultCode::TooFar.payload());
        for _ in 0..3 {
            let mut runtime = runtime.lock().await;
            let p = runtime.read_actor_position(actor).map_err(debug)?;
            runtime
                .borrow_movement_position()
                .commit_cardinal(
                    p,
                    crate::foundation::MovementLocalPosition {
                        x: p.position().x - 1,
                        ..p.position()
                    },
                )
                .map_err(debug)?;
        }
        assert_eq!(
            runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            original
        );
        assert_eq!(a.trade(offer_a).await?, ResultCode::Offered.payload());
        let logged_out = b.command(23, &[]).await?;
        assert_eq!(
            oteryn_protocol_oteryn::logout::decode_logout_result(&logged_out).map_err(debug)?,
            oteryn_protocol_oteryn::logout::LogoutResult::Accepted
        );
        // Explicit native logout cancels the partner before releasing the lease.
        loop {
            let (_, state, _) = owner
                .trade_observation(admitted[0].game_session_id)
                .await
                .ok_or("after logout")?;
            if state.phase == 4 {
                assert_eq!(state.reason, 2);
                break;
            }
            tokio::task::yield_now().await;
        }
        a.close().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (_, result) = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        join_two(serving, driving),
    )
    .await?;
    result?;
    let request = owner
        .states
        .player_trades
        .completed_request()
        .ok_or("committed native request")?;
    let restarted = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
    assert!(restarted.maintain_ready_once().await?);
    let recovered = restarted
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    assert_eq!(
        restarted
            .reconcile_player_trade(&recovered, request)
            .await?,
        Ok(crate::durability::player_trade::TradeReceipt::AlreadyCommitted)
    );
    for (current, expected) in currents.iter().zip([93, 91]) {
        let character =
            crate::domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
                .map_err(debug)?;
        let backpack = restarted
            .read_character_backpack(&recovered, character)
            .await?
            .ok_or("restart backpack")?;
        assert_eq!(backpack.entries.len(), 1);
        assert_eq!(backpack.entries[0].item.item_instance_id, id(expected));
        assert_eq!(backpack.entries[0].item.quantity, 3);
    }
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_audit_outbox a JOIN game_player_trades t ON t.event_id=a.event_id").fetch_one(&h.pool).await?,1);
    Ok(())
}
struct Peer {
    writer: std::sync::Arc<Mutex<tokio::io::WriteHalf<tokio::io::DuplexStream>>>,
    frames: tokio::sync::mpsc::Receiver<std::io::Result<Vec<u8>>>,
    reader: tokio::task::JoinHandle<()>,
    next: u64,
    last_inventory: Option<item_view::CharacterInventory>,
    last_trade: Option<Vec<u8>>,
    trade_revision: u64,
    sequence: u64,
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.reader.abort();
    }
}
impl Peer {
    async fn open(stream: tokio::io::DuplexStream) -> TestResult<Self> {
        let (mut read, write) = tokio::io::split(stream);
        let writer = std::sync::Arc::new(Mutex::new(write));
        let ack_writer = std::sync::Arc::clone(&writer);
        let (tx, frames) = tokio::sync::mpsc::channel(256);
        // Real peers keep answering liveness even while the other player is deciding.
        let reader = tokio::spawn(async move {
            loop {
                let frame = read_frame(&mut read).await;
                if let Ok(bytes) = &frame
                    && let Ok(envelope) = decode_wire_envelope(bytes)
                    && envelope.message_type() == MessageType::LivenessProbe
                {
                    let ack = oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload())
                        .and_then(|probe| oteryn_protocol_oteryn::encode_liveness_ack(1, probe, 0));
                    let Ok(ack) = ack else {
                        break;
                    };
                    if write_frame(&mut *ack_writer.lock().await, &ack)
                        .await
                        .is_err()
                    {
                        break;
                    }
                    continue;
                }
                let failed = frame.is_err();
                if tx.send(frame).await.is_err() || failed {
                    break;
                }
            }
        });
        let mut peer = Self {
            writer,
            frames,
            reader,
            next: 1,
            last_inventory: None,
            last_trade: None,
            trade_revision: 1,
            sequence: 0,
        };
        loop {
            let frame = peer.read().await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        Ok(peer)
    }
    async fn read(&mut self) -> TestResult<Vec<u8>> {
        let frame = self.frames.recv().await.ok_or("peer reader ended")??;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.server_sequence() != 0 {
            self.sequence += 1;
            assert_eq!(envelope.server_sequence(), self.sequence);
        }
        Ok(frame)
    }
    async fn close(&mut self) -> TestResult {
        self.writer.lock().await.shutdown().await?;
        Ok(())
    }
    async fn send(
        &mut self,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> TestResult<(CommandStatus, Vec<u8>)> {
        let frame = encode_client_command(
            1,
            &ClientCommandValue {
                command_id: id,
                command_type: kind,
                payload,
            },
        )
        .map_err(debug)?;
        write_frame(&mut *self.writer.lock().await, &frame).await?;
        loop {
            let frame = self.read().await?;
            let e = decode_wire_envelope(&frame).map_err(debug)?;
            if e.message_type() == MessageType::CommandResult {
                let r = decode_command_result(e.payload()).map_err(debug)?;
                assert_eq!(r.command_id, id);
                return Ok((r.status, r.payload.to_vec()));
            }
            self.capture(&frame)?;
        }
    }
    fn capture(&mut self, frame: &[u8]) -> TestResult {
        let e = decode_wire_envelope(frame).map_err(debug)?;
        if e.message_type() == MessageType::StateDelta {
            let d = decode_state_delta(e.payload()).map_err(debug)?;
            if d.domain_id == wire::STATE_DOMAIN_PLAYER_TRADE {
                assert_eq!(d.base_revision, self.trade_revision);
                assert!(d.new_revision > self.trade_revision);
                self.trade_revision = d.new_revision;
                self.last_trade = Some(d.payload.to_vec());
            }
            if d.domain_id == 9 {
                self.last_inventory =
                    Some(item_view::decode_character_inventory(d.payload).map_err(debug)?);
            }
        }
        Ok(())
    }
    async fn command(&mut self, kind: u32, payload: &[u8]) -> TestResult<Vec<u8>> {
        let r = self.send(self.next, kind, payload).await?;
        self.next += 1;
        assert_eq!(
            r.0,
            CommandStatus::Accepted,
            "command {kind}, id {}, result {:?}",
            self.next - 1,
            r.1
        );
        Ok(r.1)
    }
    async fn trade(&mut self, intent: Intent) -> TestResult<Vec<u8>> {
        let r = self
            .command(78, &wire::encode_intent(intent).map_err(debug)?)
            .await?;
        Ok(r)
    }
    async fn trade_state(&mut self, state: &wire::State) -> TestResult {
        let expected = wire::encode_state(state).map_err(debug)?;
        while self.last_trade.as_ref() != Some(&expected) {
            let frame = self.read().await?;
            self.capture(&frame)?;
        }
        Ok(())
    }
    async fn inventory(&mut self, definition: std::num::NonZeroU32) -> TestResult {
        while self.last_inventory.as_ref().is_none_or(|i| {
            i.entries
                .first()
                .is_none_or(|e| e.item_definition_ref != definition)
        }) {
            let frame = self.read().await?;
            self.capture(&frame)?;
        }
        Ok(())
    }
}
