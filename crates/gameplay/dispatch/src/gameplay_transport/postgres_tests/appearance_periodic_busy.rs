//! A long holder read defers only an established observer's staged presentation.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::foundation::{
    CharacterLease, ExactActorRef, GameSessionId, MessageType, decode_wire_envelope,
};
use crate::gameplay_transport::appearance_change::observation::{Observation, ObservationError};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAttempt,
    FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::item_view::InventoryItems;
use crate::gameplay_transport::world_spatial::{
    ChannelEntities, ChannelEntity, VisibleKind, WorldSpatialObservation,
};
use crate::gameplay_transport::{COMMAND_HOLDER_WINDOW, ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityFields, decode_world_spatial_entities_delta_with_fields,
};
use oteryn_protocol_oteryn::{appearance, decode_state_delta};
use std::cell::Cell;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::sync::Notify;

#[derive(Clone, Copy)]
enum Case {
    Busy,
    Storage,
    Authority,
}

#[test]
fn appearance_periodic_holder_busy_postgres_keeps_live_peer_and_discards_staged_cursors()
-> TestResult {
    run_case(Case::Busy)
}

#[test]
fn appearance_periodic_storage_failure_postgres_still_disconnects() -> TestResult {
    run_case(Case::Storage)
}

#[test]
fn appearance_periodic_authority_failure_postgres_still_disconnects() -> TestResult {
    run_case(Case::Authority)
}

fn run_case(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "periodic appearance test thread panicked")??;
    Ok(())
}

// The real appearance owner and holder are exercised through serve_admitted.
// Other projections are fixed fixture observations so contention in a different
// periodic owner cannot decide whether the appearance boundary was reached.
struct ProjectionOwner<'o, 'a, 'f, 's> {
    owner: &'o ComposedFreshAdmission<'a, 'f, 's>,
    baseline: WorldSpatialObservation,
    channel: ChannelEntities,
    clock_ms: Cell<u64>,
    item_offset: Cell<i32>,
    arm: Cell<bool>,
    entered: Notify,
    resume: Notify,
    busy: Cell<u32>,
    unavailable: Cell<u32>,
}

impl FreshAdmissionAuthority for ProjectionOwner<'_, '_, '_, '_> {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(self.baseline)
    }

    async fn observe_visible_entities(&self, _: ExactActorRef) -> Option<ChannelEntities> {
        let mut channel = self.channel.clone();
        let offset = self.item_offset.get();
        if offset != 0 {
            let mut position = self.baseline.actor_position;
            position.x += offset;
            channel.entities.push(ChannelEntity {
                display_name: None,
                health_percent: 100,
                direction: oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                kind: VisibleKind::GroundItem {
                    item_definition_ref: std::num::NonZeroU32::new(1)?,
                },
                identity: id(120),
                // Ground items carry no actor incarnation on the wire.
                generation: 0,
                position,
                revision: u64::try_from(offset).ok()?,
            });
        }
        Some(channel)
    }

    async fn observe_character_inventory(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<InventoryItems> {
        Some(InventoryItems::default())
    }

    fn world_clock_utc_ms(&self) -> Option<u64> {
        Some(self.clock_ms.get())
    }

    async fn appearance_observation_projected(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        empty_outfit: bool,
        entities: Vec<appearance::EntityRef>,
    ) -> Option<Observation> {
        self.owner
            .appearance_observation_projected(actor, session, empty_outfit, entities)
            .await
    }

    async fn appearance_observation_projected_result(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        empty_outfit: bool,
        entities: Vec<appearance::EntityRef>,
    ) -> Result<Observation, ObservationError> {
        if self.arm.replace(false) {
            self.entered.notify_one();
            self.resume.notified().await;
        }
        let result = self
            .owner
            .appearance_observation_projected_result(actor, session, empty_outfit, entities)
            .await;
        match result {
            Err(ObservationError::HolderBusy) => self.busy.set(self.busy.get() + 1),
            Err(ObservationError::Unavailable) => self.unavailable.set(self.unavailable.get() + 1),
            Ok(_) => {}
        }
        result
    }
}

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let tag = match case {
            Case::Busy => "appearance_periodic_busy",
            Case::Storage => "appearance_periodic_storage",
            Case::Authority => "appearance_periodic_authority",
        };
        let h = Harness::create_appearance(admin, tag).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let mut packet = cosmetics_test_packet();
        packet["authoring"]["document"]["content_revision"] = "content-1".into();
        let (controller, pin) =
            activated_with_optional_cosmetics(world, None, Some(&serde_json::to_vec(&packet)?));
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        deps.initialize(&owner).await;
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = item_fence_of(current);
        let offered = crate::gameplay_transport::appearance_change::offer::offered(
            owner.offered_capabilities(),
            owner.appearance_available(),
        );
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offered, &[4, 6, 19, 25])
                .ok_or("appearance capabilities")?;
        let projection = ProjectionOwner {
            owner: &owner,
            baseline: owner.observe(actor).await.ok_or("baseline")?,
            channel: owner
                .observe_visible_entities(actor)
                .await
                .ok_or("channel")?,
            clock_ms: Cell::new(0),
            item_offset: Cell::new(0),
            arm: Cell::new(false),
            entered: Notify::new(),
            resume: Notify::new(),
            busy: Cell::new(0),
            unavailable: Cell::new(0),
        };
        assert_eq!(projection.channel.entities.len(), 1);

        if matches!(case, Case::Busy) {
            // Initial join still fails closed on the same exhausted holder budget.
            let held = owner.root.try_issue_semantic_pass().map_err(debug)?;
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            let end = tokio::time::timeout(
                IDLE_LIVENESS.interval,
                serve_admitted(&mut server, admitted, &projection, IDLE_LIVENESS),
            )
            .await?;
            assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
            drop(server);
            let mut bytes = Vec::new();
            client.read_to_end(&mut bytes).await?;
            assert!(bytes.is_empty(), "failed join published a partial snapshot");
            drop(held);
        }

        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serving = async {
            let end = serve_admitted(&mut server, admitted, &projection, IDLE_LIVENESS).await;
            drop(server);
            end
        };
        let (peer_projection, peer_owner, peer_harness) = (&projection, &owner, &h);
        let peer = async move {
            let (projection, owner, h) = (peer_projection, peer_owner, peer_harness);
            let initial = read_snapshot(&mut client).await?;
            let binding = initial.bindings.first().ok_or("initial binding")?;
            assert_eq!(initial.bindings.len(), 1);
            assert_eq!(initial.high_water, 1);
            assert_eq!(binding.appearance_ref, 1);
            let mut probes = 0;
            if matches!(case, Case::Busy) {
                // Begin the contention window with a real live peer's ACK ready.
                loop {
                    let bytes = read_frame(&mut client).await?;
                    let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                    if envelope.message_type() == MessageType::LivenessProbe {
                        projection.item_offset.set(1);
                        projection.arm.set(true);
                        acknowledge(&mut client, envelope.payload(), 0).await?;
                        probes += 1;
                        break;
                    }
                    assert_ne!(envelope.message_type(), MessageType::StateDelta);
                }
            } else {
                projection.arm.set(true);
            }
            projection.entered.notified().await;
            assert!(owner.runtime.try_lock().is_ok());
            assert!(owner.states.spell_states.try_lock().is_ok());
            match case {
                Case::Busy => {
                    let held = owner.root.try_issue_semantic_pass().map_err(debug)?;
                    let held_at = tokio::time::Instant::now();
                    install_temporary(owner, actor, session).await?;
                    projection.clock_ms.set(150_000);
                    projection.resume.notify_one();
                    let mut sequence = 0;
                    // A due independent clock frame must be emitted while the real
                    // holder is still occupied; no staged mapping/spatial frame may be.
                    loop {
                        let bytes = read_frame(&mut client).await?;
                        let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                        if envelope.message_type() == MessageType::LivenessProbe {
                            acknowledge(&mut client, envelope.payload(), sequence).await?;
                            probes += 1;
                            continue;
                        }
                        assert_eq!(envelope.message_type(), MessageType::StateDelta);
                        sequence += 1;
                        assert_eq!(envelope.server_sequence(), sequence);
                        let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                        assert_eq!(delta.domain_id, 18, "staged projection escaped while Busy");
                        assert!(
                            projection.busy.get() > 0,
                            "clock bypassed the busy appearance read"
                        );
                        break;
                    }
                    // This is the contract being exercised, not a readiness sleep:
                    // retain the holder beyond the entire existing command window.
                    tokio::time::sleep_until(
                        held_at + COMMAND_HOLDER_WINDOW + std::time::Duration::from_millis(100),
                    )
                    .await;
                    assert!(held_at.elapsed() > COMMAND_HOLDER_WINDOW);
                    assert!(owner.runtime.try_lock().is_ok());
                    assert!(owner.states.spell_states.try_lock().is_ok());
                    // The previously staged position must also be discarded.
                    projection.item_offset.set(2);
                    drop(held);
                    let mut mapped = false;
                    let mut spatial = false;
                    while !spatial || probes < 2 {
                        let bytes = read_frame(&mut client).await?;
                        let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                        if envelope.message_type() == MessageType::LivenessProbe {
                            acknowledge(&mut client, envelope.payload(), sequence).await?;
                            probes += 1;
                            continue;
                        }
                        assert_eq!(envelope.message_type(), MessageType::StateDelta);
                        sequence += 1;
                        assert_eq!(envelope.server_sequence(), sequence);
                        let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                        assert_eq!((delta.base_revision, delta.new_revision), (1, 2));
                        match delta.domain_id {
                            21 => {
                                assert!(!mapped && !spatial);
                                let update =
                                    appearance::Mappings::decode(delta.payload).map_err(debug)?;
                                assert_eq!(update.upserts.len(), 1);
                                assert!(update.forgets.is_empty());
                                assert_eq!(
                                    update.upserts[0].appearance_ref,
                                    binding.appearance_ref
                                );
                                assert!(matches!(
                                    update.upserts[0].look,
                                    appearance::Look::Outfit {
                                        look_type: 1593,
                                        ..
                                    }
                                ));
                                mapped = true;
                            }
                            1 => {
                                assert!(mapped && !spatial);
                                let (update, _) = decode_world_spatial_entities_delta_with_fields(
                                    delta.payload,
                                    EntityFields::selected(
                                        admitted.continuity.selected_capabilities.as_slice(),
                                    ),
                                )
                                .map_err(debug)?;
                                assert_eq!(update.enter.len(), 1);
                                assert_eq!(update.update.len(), 1);
                                assert!(update.leave.is_empty());
                                assert_eq!(
                                    update.enter[0].position.x,
                                    projection.baseline.actor_position.x + 2
                                );
                                let oteryn_protocol_oteryn::world_spatial_entities::EntityDetail::Object { item_handle: Some(handle), .. } = update.enter[0].detail else {
                                    return Err("new ground item has no handle".into());
                                };
                                assert_eq!(
                                    handle.get(),
                                    1,
                                    "deferred candidate consumed an item handle"
                                );
                                spatial = true;
                            }
                            _ => return Err("unexpected projection domain".into()),
                        }
                    }
                    assert_eq!(sequence, 3);
                    assert_eq!(projection.unavailable.get(), 0);
                }
                Case::Storage => {
                    // A real SELECT failure/deadline is not a retryable holder miss.
                    let mut locked = h.pool.begin().await?;
                    sqlx::query(
                        "LOCK TABLE game_character_store_cosmetics IN ACCESS EXCLUSIVE MODE",
                    )
                    .execute(&mut *locked)
                    .await?;
                    projection.resume.notify_one();
                    assert!(
                        read_frame(&mut client).await.is_err(),
                        "storage failure kept serving"
                    );
                    locked.rollback().await?;
                }
                Case::Authority => {
                    owner.runtime.lock().await.retire_owner_cycle();
                    projection.resume.notify_one();
                    assert!(
                        read_frame(&mut client).await.is_err(),
                        "retired owner kept serving"
                    );
                }
            }
            client.shutdown().await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        let (end, observed) = tokio::time::timeout(
            IDLE_LIVENESS.interval * 4,
            super::fixture::join_two(serving, peer),
        )
        .await?;
        observed?;
        let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
            return Err(format!("responsive appearance peer lost control: {end:?}").into());
        };
        match case {
            Case::Busy => {
                assert_eq!(ended.continuity.server_sequence, 3);
                assert_eq!(ended.continuity.spatial_revision, 2);
                assert!(projection.busy.get() > 0);
            }
            Case::Storage | Case::Authority => {
                assert_eq!(projection.busy.get(), 0);
                assert_eq!(projection.unavailable.get(), 1);
                assert_eq!(ended.continuity.server_sequence, 0);
                assert_eq!(ended.continuity.spatial_revision, 1);
            }
        }
        drop(projection);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn install_temporary(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult {
    let member = owner
        .active_generation
        .ok_or("generation")?
        .native_gameplay()
        .ok_or("native")?
        .spell_appearances()
        .ok_or("appearances")?
        .for_look_type(1593)
        .ok_or("temporary outfit")?;
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let state = states.get_mut(&runtime, actor, session).ok_or("player")?;
    crate::spell::actor_conditions::apply_appearance(
        state,
        member.selection(),
        member.look_type(),
        600_000,
        owner.owner_now().get() / 1000,
    )
    .map_err(debug)?;
    state.advance_batch_revision().map_err(debug)?;
    Ok(())
}

async fn read_snapshot(client: &mut DuplexStream) -> TestResult<appearance::Snapshot> {
    let mut appearance = None;
    loop {
        let bytes = read_frame(client).await?;
        let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
        match envelope.message_type() {
            MessageType::SnapshotChunk => {
                let (_, body) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk_framing(envelope.payload())
                        .map_err(debug)?;
                for domain in oteryn_protocol_oteryn::decode_snapshot_body(body).map_err(debug)? {
                    if domain.domain_id == 21 {
                        appearance =
                            Some(appearance::Snapshot::decode(domain.payload).map_err(debug)?);
                    }
                }
            }
            MessageType::SnapshotCommit => {
                return appearance.ok_or_else(|| "appearance snapshot missing".into());
            }
            _ => {}
        }
    }
}

async fn read_frame(client: &mut DuplexStream) -> TestResult<Vec<u8>> {
    let length = usize::try_from(client.read_u32().await?)?;
    if length > 1 << 20 {
        return Err("oversized test frame".into());
    }
    let mut bytes = vec![0; length];
    client.read_exact(&mut bytes).await?;
    Ok(bytes)
}

async fn acknowledge(client: &mut DuplexStream, payload: &[u8], sequence: u64) -> TestResult {
    let probe = oteryn_protocol_oteryn::decode_liveness_probe(payload).map_err(debug)?;
    let ack = oteryn_protocol_oteryn::encode_liveness_ack(1, probe, sequence).map_err(debug)?;
    client.write_u32(u32::try_from(ack.len())?).await?;
    client.write_all(&ack).await?;
    Ok(())
}
